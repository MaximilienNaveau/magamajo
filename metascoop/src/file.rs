use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Move a file from oldpath to newpath
/// First tries a simple rename, then falls back to copy+delete if needed
pub fn move_file(oldpath: &Path, newpath: &Path) -> Result<()> {
    // Try the normal method using rename
    match fs::rename(oldpath, newpath) {
        Ok(_) => Ok(()),
        Err(_) => {
            // If rename fails (e.g., across filesystems), do copy+delete
            fs::copy(oldpath, newpath)
                .with_context(|| {
                    format!(
                        "Failed to copy {} to {}",
                        oldpath.display(),
                        newpath.display()
                    )
                })?;

            fs::remove_file(oldpath)
                .with_context(|| format!("Failed to remove {}", oldpath.display()))?;

            Ok(())
        }
    }
}
