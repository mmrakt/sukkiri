use crate::model::ScannedItem;
use anyhow::Result;

use std::process::Command;

pub fn delete_items(items: &[ScannedItem]) -> Result<()> {
    if items.is_empty() {
        return Ok(());
    }

    let mut file_paths = Vec::new();
    let mut docker_ids = Vec::new();

    for item in items {
        let path_str = item.path.to_string_lossy();
        if path_str.starts_with("docker://") {
            // Format: docker://<ID>/<Name>
            if let Some(rest) = path_str.strip_prefix("docker://") {
                // Extract ID (part before the first slash)
                let id = rest.split('/').next().unwrap_or(rest);
                docker_ids.push(id.to_string());
            }
        } else {
            file_paths.push(&item.path);
        }
    }

    // 1. Delete Docker images (Permanent!)
    for id in docker_ids {
        let output = Command::new("docker").args(["rmi", &id]).output();

        match output {
            Ok(out) => {
                if !out.status.success() {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    return Err(anyhow::anyhow!(
                        "Failed to remove Docker image {id}.\nStdout: {stdout}\nStderr: {stderr}"
                    ));
                }
            }
            Err(e) => return Err(anyhow::anyhow!("Failed to execute docker rmi: {e}")),
        }
    }

    // 2. Permanently delete files
    if !file_paths.is_empty() {
        for path in file_paths {
            if path.is_dir() {
                let _ = std::fs::remove_dir_all(path);
            } else {
                let _ = std::fs::remove_file(path);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ScannedItem;
    use std::fs::File;
    use std::time::SystemTime;
    use tempfile::tempdir;

    #[test]
    fn permanent_delete_logic() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test_file.txt");
        File::create(&file_path)?;

        assert!(file_path.exists());

        let item = ScannedItem {
            path: file_path.clone(),
            size: 0,
            modified: SystemTime::now(),
        };

        delete_items(&[item])?;

        assert!(!file_path.exists());
        Ok(())
    }

    #[test]
    fn move_to_trash_empty_list() -> Result<()> {
        let items: Vec<ScannedItem> = vec![];
        delete_items(&items)?;
        Ok(())
    }
}
