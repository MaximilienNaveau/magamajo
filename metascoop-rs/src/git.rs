use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn clone_repo(git_url: &str) -> Result<PathBuf> {
    let temp_dir = tempfile::tempdir()
        .context("Failed to create temporary directory")?;
    
    let dir_path = temp_dir.path().to_path_buf();
    
    let output = Command::new("git")
        .args(["clone", git_url, dir_path.to_str().unwrap()])
        .output()
        .context("Failed to execute git clone")?;

    if !output.status.success() {
        anyhow::bail!(
            "Git clone failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Keep the directory by consuming the TempDir without cleanup
    #[allow(deprecated)]
    let _ = temp_dir.into_path();

    Ok(dir_path)
}

pub fn get_changed_file_names(repo_path: &Path) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["diff", "--name-only"])
        .current_dir(repo_path)
        .output()
        .context("Failed to execute git diff")?;

    if !output.status.success() {
        anyhow::bail!(
            "Git diff failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let paths: Vec<String> = output_str
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok(paths)
}
