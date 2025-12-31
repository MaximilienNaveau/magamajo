use anyhow::{Context, Result};
use octocrab::models::{repos::Release, repos::Asset};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use unicode_normalization::UnicodeNormalization;
use url::Url;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppInfo {
    pub git: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub anti_features: Vec<String>,
    #[serde(skip)]
    pub release_description: String,
    #[serde(default)]
    pub license: String,
    #[serde(skip)]
    pub key_name: String,
    #[serde(skip)]
    pub repo_author: String,
}

impl AppInfo {
    pub fn app_name(&self) -> &str {
        if !self.key_name.is_empty() {
            &self.key_name
        } else {
            &self.name
        }
    }

    pub fn author_name(&self) -> &str {
        if !self.author.is_empty() {
            &self.author
        } else {
            &self.repo_author
        }
    }
}

#[derive(Debug, Clone)]
pub struct Repo {
    pub author: String,
    pub name: String,
    #[allow(dead_code)]
    pub host: String,
}

pub fn parse_app_file(filepath: &Path) -> Result<Vec<AppInfo>> {
    let content = fs::read_to_string(filepath)
        .with_context(|| format!("Failed to read app file: {}", filepath.display()))?;

    let mut apps: HashMap<String, AppInfo> = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse YAML from {}", filepath.display()))?;

    let mut list = Vec::new();
    for (key, mut app) in apps.drain() {
        app.key_name = key.clone();

        let url = Url::parse(&app.git).with_context(|| {
            format!("Invalid git URL '{}' for app with key '{}'", app.git, key)
        })?;

        let path_segments: Vec<&str> = url
            .path_segments()
            .ok_or_else(|| anyhow::anyhow!("Cannot extract path from URL"))?
            .collect();

        if !path_segments.is_empty() {
            app.repo_author = path_segments[0].to_string();
        }

        list.push(app);
    }

    Ok(list)
}

pub fn repo_info(repo_url: &str) -> Result<Repo> {
    let url = Url::parse(repo_url)?;

    let path_segments: Vec<&str> = url
        .path_segments()
        .ok_or_else(|| anyhow::anyhow!("Cannot extract path from URL"))?
        .collect();

    if path_segments.len() < 2 {
        anyhow::bail!("URL path must have at least 2 segments");
    }

    Ok(Repo {
        author: path_segments[0].to_string(),
        name: path_segments[1].to_string(),
        host: url.host_str().unwrap_or("").trim_start_matches("www.").to_string(),
    })
}

pub fn find_apk_release(release: &Release) -> Option<Asset> {
    release.assets.iter().find(|asset| {
        asset.state == "uploaded" && asset.name.ends_with(".apk")
    }).cloned()
}

pub fn generate_release_filename(app_name: &str, tag_name: &str) -> String {
    let normal_name = format!("{}_{}.apk", app_name, tag_name);

    // Normalize Unicode characters
    let normalized: String = normal_name.nfd().collect();

    // Remove combining marks and keep only allowed characters
    normalized
        .chars()
        .filter_map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' {
                Some(c)
            } else if c.is_whitespace() {
                Some('_')
            } else if !c.is_ascii() {
                // Skip non-ASCII characters that aren't alphanumeric
                None
            } else {
                None
            }
        })
        .collect()
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RepoIndex {
    pub repo: serde_json::Value,
    pub requests: serde_json::Value,
    pub apps: Vec<serde_json::Value>,
    pub packages: HashMap<String, Vec<PackageInfo>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackageInfo {
    pub added: i64,
    pub apk_name: String,
    pub hash: String,
    pub hash_type: String,
    pub min_sdk_version: i32,
    #[serde(default)]
    pub nativecode: Vec<String>,
    pub package_name: String,
    pub sig: String,
    pub signer: String,
    pub size: i64,
    pub target_sdk_version: i32,
    #[serde(default)]
    pub version_code: i32,
    pub version_name: String,
}

impl RepoIndex {
    pub fn find_latest_package(&self, pkg_name: &str) -> Option<PackageInfo> {
        let pkgs = self.packages.get(pkg_name)?;
        if pkgs.is_empty() {
            return None;
        }

        let mut sorted = pkgs.clone();
        sorted.sort_by(|a, b| {
            if a.version_code != b.version_code {
                a.version_code.cmp(&b.version_code)
            } else {
                a.version_name.cmp(&b.version_name)
            }
        });

        sorted.last().cloned()
    }

    pub fn read_index(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read index file: {}", path.display()))?;
        
        let index: RepoIndex = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse JSON from {}", path.display()))?;
        
        Ok(index)
    }
}

pub fn read_meta_file(path: &Path) -> Result<HashMap<String, serde_yaml::Value>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read meta file: {}", path.display()))?;

    let data: HashMap<String, serde_yaml::Value> = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse YAML from {}", path.display()))?;

    Ok(data)
}

pub fn write_meta_file(path: &Path, data: &HashMap<String, serde_yaml::Value>) -> Result<()> {
    let tmp_path = path.with_extension("yml.tmp");
    
    let content = serde_yaml::to_string(data)?;
    fs::write(&tmp_path, content)
        .with_context(|| format!("Failed to write to temp file: {}", tmp_path.display()))?;

    fs::rename(&tmp_path, path)
        .with_context(|| format!("Failed to rename {} to {}", tmp_path.display(), path.display()))?;

    Ok(())
}

#[derive(Debug, Default)]
pub struct RepoMetadata {
    pub screenshots: Vec<PathBuf>,
}

pub fn find_metadata(cloned_repo_path: &Path) -> Result<RepoMetadata> {
    let mut metadata = RepoMetadata::default();
    
    for entry in walkdir::WalkDir::new(cloned_repo_path) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let path_str = path.to_string_lossy().to_lowercase();

        if path_str.contains("screenshot") && is_image_file(path) {
            metadata.screenshots.push(path.to_path_buf());
        }
    }

    Ok(metadata)
}

fn is_image_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy().to_lowercase();
        matches!(ext.as_str(), "png" | "jpg" | "jpeg")
    } else {
        false
    }
}

pub fn has_significant_changes(old: &RepoIndex, new: &RepoIndex) -> (String, bool) {
    // This is a simplified version - you might want to use a proper diff crate
    // For now, we'll just compare the packages
    if old.packages != new.packages {
        return ("packages".to_string(), true);
    }
    
    ("".to_string(), false)
}
