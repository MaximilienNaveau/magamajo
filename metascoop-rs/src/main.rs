mod apps;
mod file;
mod git;
mod md;

use anyhow::{Context, Result};
use clap::Parser;
use log::{error, info};
use octocrab::Octocrab;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to apps.yaml file
    #[arg(short = 'a', long, default_value = "apps.yaml")]
    apps_path: PathBuf,

    /// Path to fdroid "repo" directory
    #[arg(short = 'r', long, default_value = "fdroid/repo")]
    repo_dir: PathBuf,

    /// GitHub personal access token
    #[arg(short = 'p', long, env = "GITHUB_TOKEN")]
    personal_access_token: Option<String>,

    /// Debug mode won't run the fdroid command
    #[arg(short = 'd', long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    println!("::group::Initializing");

    let apps_list = apps::parse_app_file(&args.apps_path)
        .context("Failed to parse app file")?;

    let github = if let Some(token) = &args.personal_access_token {
        Octocrab::builder()
            .personal_token(token.clone())
            .build()?
    } else {
        Octocrab::builder().build()?
    };

    let mut have_error = false;

    let fdroid_index_path = args.repo_dir.join("index-v1.json");

    let initial_fdroid_index = apps::RepoIndex::read_index(&fdroid_index_path)
        .context("Failed to read F-Droid repo index")?;

    fs::create_dir_all(&args.repo_dir)
        .context("Failed to create repo directory")?;

    println!("::endgroup::");

    // Map of apk_name -> AppInfo
    let mut apk_info_map: HashMap<String, apps::AppInfo> = HashMap::new();

    for mut app in apps_list {
        println!("App: {}/{}", app.author_name(), app.app_name());

        let repo = apps::repo_info(&app.git)
            .with_context(|| format!("Failed to get repo info from URL: {}", app.git))?;

        info!("Looking up {}/{} on GitHub", repo.author, repo.name);

        match github.repos(&repo.author, &repo.name).get().await {
            Ok(gh_repo) => {
                if let Some(desc) = gh_repo.description {
                    app.summary = desc;
                }

                if let Some(license) = gh_repo.license {
                    if let Some(spdx_id) = license.spdx_id {
                        app.license = spdx_id;
                    }
                }

                info!("Data from GitHub: summary={:?}, license={:?}", app.summary, app.license);
            }
            Err(e) => {
                error!("Error while looking up repo: {}", e);
            }
        }

        let releases = match github
            .repos(&repo.author, &repo.name)
            .releases()
            .list()
            .per_page(100)
            .send()
            .await
        {
            Ok(page) => {
                let mut all_releases = page.items;
                // For simplicity, we're only getting the first page
                // In production, you'd want to paginate through all results
                all_releases
            }
            Err(e) => {
                error!("Error while listing repo releases for {:?}: {}", app.git, e);
                have_error = true;
                continue;
            }
        };

        info!("Received {} releases", releases.len());

        for release in releases {
            println!("::group::Release {}", release.tag_name);

            if release.prerelease {
                info!("Skipping prerelease {:?}", release.tag_name);
                println!("::endgroup::");
                continue;
            }

            if release.draft {
                info!("Skipping draft {:?}", release.tag_name);
                println!("::endgroup::");
                continue;
            }

            if release.tag_name.is_empty() {
                info!("Skipping release with empty tag name");
                println!("::endgroup::");
                continue;
            }

            info!("Working on release with tag name {:?}", release.tag_name);

            let apk = match apps::find_apk_release(&release) {
                Some(asset) => asset,
                None => {
                    info!("Couldn't find a release asset with extension \".apk\"");
                    println!("::endgroup::");
                    continue;
                }
            };

            let app_name = apps::generate_release_filename(app.app_name(), &release.tag_name);
            info!("Target APK name: {}", app_name);

            let mut app_clone = app.clone();
            app_clone.release_description = release.body.clone().unwrap_or_default();

            if !app_clone.release_description.is_empty() {
                info!("Release notes: {}", app_clone.release_description);
            }

            apk_info_map.insert(app_name.clone(), app_clone);

            let app_target_path = args.repo_dir.join(&app_name);

            if app_target_path.exists() {
                info!("Already have APK for version {:?} at {:?}", release.tag_name, app_target_path);
                println!("::endgroup::");
                continue;
            }

            info!("Downloading APK {:?} from release {:?} to {:?}", apk.name, release.tag_name, app_target_path);

            match download_asset(&github, &repo.author, &repo.name, apk.id.0, &app_target_path).await {
                Ok(_) => {
                    info!("Successfully downloaded app for version {:?}", release.tag_name);
                }
                Err(e) => {
                    error!("Error while downloading app: {}", e);
                    have_error = true;
                }
            }

            println!("::endgroup::");
        }
    }

    if !args.debug {
        println!("::group::F-Droid: Creating metadata stubs");

        let fdroid_dir = args.repo_dir.parent()
            .ok_or_else(|| anyhow::anyhow!("Repo dir has no parent"))?;

        let status = Command::new("fdroid")
            .args(["update", "--pretty", "--create-metadata", "--delete-unknown"])
            .current_dir(fdroid_dir)
            .status()
            .context("Failed to run fdroid update")?;

        if !status.success() {
            error!("Error while running \"fdroid update -c\"");
            println!("::endgroup::");
            std::process::exit(1);
        }

        println!("::endgroup::");
    }

    println!("Filling in metadata");

    let fdroid_index = apps::RepoIndex::read_index(&fdroid_index_path)
        .context("Failed to read F-Droid repo index after update")?;

    let mut to_remove_paths: Vec<PathBuf> = Vec::new();

    let metadata_dir = args.repo_dir.parent()
        .ok_or_else(|| anyhow::anyhow!("Repo dir has no parent"))?
        .join("metadata");

    for entry in walkdir::WalkDir::new(&metadata_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        
        if !path.is_file() || !path.extension().map_or(false, |e| e == "yml") {
            continue;
        }

        let pkg_name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        println!("::group::{}", pkg_name);

        info!("Working on {:?}", pkg_name);

        let mut meta = match apps::read_meta_file(path) {
            Ok(m) => m,
            Err(e) => {
                error!("Reading meta file {:?}: {}", path, e);
                println!("::endgroup::");
                continue;
            }
        };

        let latest_package = match fdroid_index.find_latest_package(&pkg_name) {
            Some(p) => p,
            None => {
                println!("::endgroup::");
                continue;
            }
        };

        info!("The latest version is {:?} with versionCode {}", latest_package.version_name, latest_package.version_code);

        let apk_info = match apk_info_map.get(&latest_package.apk_name) {
            Some(info) => info,
            None => {
                info!("Cannot find apk info for {:?}", latest_package.apk_name);
                println!("::endgroup::");
                continue;
            }
        };

        // Update metadata
        set_non_empty(&mut meta, "AuthorName", apk_info.author_name());
        
        let friendly_name = if !apk_info.name.is_empty() {
            &apk_info.name
        } else {
            apk_info.app_name()
        };
        set_non_empty(&mut meta, "Name", friendly_name);
        set_non_empty(&mut meta, "SourceCode", &apk_info.git);
        set_non_empty(&mut meta, "License", &apk_info.license);
        set_non_empty(&mut meta, "Description", &apk_info.description);

        let mut summary = apk_info.summary.clone();
        const MAX_SUMMARY_LENGTH: usize = 80;
        if summary.len() > MAX_SUMMARY_LENGTH {
            summary.truncate(MAX_SUMMARY_LENGTH - 3);
            summary.push_str("...");
            info!("Truncated summary to length of {} (max length)", summary.len());
        }
        set_non_empty(&mut meta, "Summary", &summary);

        if !apk_info.categories.is_empty() {
            meta.insert("Categories".to_string(), serde_yaml::Value::Sequence(
                apk_info.categories.iter().map(|s| serde_yaml::Value::String(s.clone())).collect()
            ));
        }

        if !apk_info.anti_features.is_empty() {
            let anti_features = apk_info.anti_features.join(",");
            meta.insert("AntiFeatures".to_string(), serde_yaml::Value::String(anti_features));
        }

        meta.insert("CurrentVersion".to_string(), serde_yaml::Value::String(latest_package.version_name.clone()));
        meta.insert("CurrentVersionCode".to_string(), serde_yaml::Value::Number(latest_package.version_code.into()));

        info!("Set current version info to versionName={:?}, versionCode={}", latest_package.version_name, latest_package.version_code);

        if let Err(e) = apps::write_meta_file(path, &meta) {
            error!("Writing meta file {:?}: {}", path, e);
            println!("::endgroup::");
            continue;
        }

        info!("Updated metadata file {:?}", path);

        // Write changelog
        if !apk_info.release_description.is_empty() {
            let changelog_path = metadata_dir
                .join(&latest_package.package_name)
                .join("en-US")
                .join("changelogs")
                .join(format!("{}.txt", latest_package.version_code));

            if let Some(parent) = changelog_path.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::write(&changelog_path, &apk_info.release_description)?;
            info!("Wrote release notes to {:?}", changelog_path);
        }

        // Clone repo and find screenshots
        info!("Cloning git repository to search for screenshots");

        match git::clone_repo(&apk_info.git) {
            Ok(git_repo_path) => {
                match apps::find_metadata(&git_repo_path) {
                    Ok(repo_metadata) => {
                        info!("Found {} screenshots", repo_metadata.screenshots.len());

                        let screenshots_path = metadata_dir
                            .join(&latest_package.package_name)
                            .join("en-US")
                            .join("phoneScreenshots");

                        let _ = fs::remove_dir_all(&screenshots_path);

                        let mut counter = 1;
                        for screenshot in &repo_metadata.screenshots {
                            if let Some(ext) = screenshot.extension() {
                                let new_file_path = screenshots_path.join(format!("{}.{}", counter, ext.to_string_lossy()));

                                if let Some(parent) = new_file_path.parent() {
                                    fs::create_dir_all(parent)?;
                                }

                                if let Err(e) = file::move_file(screenshot, &new_file_path) {
                                    error!("Moving screenshot file {:?} to {:?}: {}", screenshot, new_file_path, e);
                                } else {
                                    info!("Wrote screenshot to {:?}", new_file_path);
                                    counter += 1;
                                }
                            }
                        }

                        to_remove_paths.push(screenshots_path);
                    }
                    Err(e) => {
                        error!("Finding metadata in git repo {:?}: {}", git_repo_path, e);
                    }
                }

                let _ = fs::remove_dir_all(&git_repo_path);
            }
            Err(e) => {
                error!("Cloning git repo from {:?}: {}", apk_info.git, e);
            }
        }

        println!("::endgroup::");
    }

    if !args.debug {
        println!("::group::F-Droid: Reading updated metadata");

        let fdroid_dir = args.repo_dir.parent()
            .ok_or_else(|| anyhow::anyhow!("Repo dir has no parent"))?;

        let status = Command::new("fdroid")
            .args(["update", "--pretty", "--delete-unknown"])
            .current_dir(fdroid_dir)
            .status()
            .context("Failed to run fdroid update")?;

        if !status.success() {
            error!("Error while running \"fdroid update\"");
            println!("::endgroup::");
            std::process::exit(1);
        }

        println!("::endgroup::");
    }

    println!("::group::Assessing changes");

    let fdroid_index = apps::RepoIndex::read_index(&fdroid_index_path)
        .context("Failed to read F-Droid repo index after final update")?;

    // Remove marked paths
    for rm_path in to_remove_paths {
        let _ = fs::remove_dir_all(&rm_path);
    }

    // Generate README
    let readme_path = args.repo_dir.parent()
        .ok_or_else(|| anyhow::anyhow!("Repo dir has no parent"))?
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Repo dir has no grandparent"))?
        .join("README.md");

    if let Err(e) = md::regenerate_readme(&readme_path, &fdroid_index) {
        error!("Error generating {:?}: {}", readme_path, e);
    }

    let (change_path, mut have_significant_changes) = apps::has_significant_changes(&initial_fdroid_index, &fdroid_index);
    
    if have_significant_changes {
        info!("The index {:?} had a significant change at JSON path {:?}", fdroid_index_path, change_path);
    } else {
        info!("The index files didn't change significantly");

        match git::get_changed_file_names(&args.repo_dir) {
            Ok(changed_files) => {
                for fname in changed_files {
                    if !fname.contains("index") {
                        have_significant_changes = true;
                        info!("File {:?} is a significant change", fname);
                    }
                }

                if !have_significant_changes {
                    info!("It doesn't look like there were any relevant changes");
                }
            }
            Err(e) => {
                error!("Getting changed files: {}", e);
            }
        }
    }

    println!("::endgroup::");

    if have_error {
        std::process::exit(1);
    }

    if !have_significant_changes {
        std::process::exit(2);
    }

    Ok(())
}

fn set_non_empty(meta: &mut HashMap<String, serde_yaml::Value>, key: &str, value: &str) {
    if !value.is_empty() || meta.get(key).and_then(|v| v.as_str()) == Some("Unknown") {
        meta.insert(key.to_string(), serde_yaml::Value::String(value.to_string()));
        info!("Set {} to {:?}", key, value);
    }
}

async fn download_asset(
    github: &Octocrab,
    owner: &str,
    repo: &str,
    asset_id: u64,
    target_path: &Path,
) -> Result<()> {
    let temp_path = target_path.with_extension("tmp");

    let asset_url = format!(
        "https://api.github.com/repos/{}/{}/releases/assets/{}",
        owner, repo, asset_id
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()?;

    let mut request = client
        .get(&asset_url)
        .header("Accept", "application/octet-stream")
        .header("User-Agent", "metascoop-rs");

    if let Some(token) = std::env::var("GITHUB_TOKEN").ok() {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let response = request.send().await?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to download asset: HTTP {}", response.status());
    }

    let mut file = tokio::fs::File::create(&temp_path).await?;
    let bytes = response.bytes().await?;
    file.write_all(&bytes).await?;
    file.sync_all().await?;

    tokio::fs::rename(&temp_path, target_path).await?;

    Ok(())
}
