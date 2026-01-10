use crate::allowlist::Allowlist;
use crate::model::ScannedItem;
use jwalk::WalkDir;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Helper function to scan a path and return total size and items.
pub fn scan_path(
    target_path: &Path,
    progress_cb: Option<&(dyn Fn() + Sync)>,
    allowlist: &Allowlist,
) -> (u64, Vec<ScannedItem>) {
    if !target_path.exists() {
        return (0, vec![]);
    }

    let entries: Vec<PathBuf> = match fs::read_dir(target_path) {
        Ok(read_dir) => read_dir.filter_map(|e| e.ok().map(|e| e.path())).collect(),
        Err(_) => vec![],
    };

    let mut items: Vec<ScannedItem> = entries
        .par_iter()
        .filter(|path| !allowlist.is_allowed(path))
        .map(|path| {
            if let Some(cb) = progress_cb {
                cb();
            }
            calculate_item_stats(path)
        })
        .collect();

    let total_size: u64 = items.iter().map(|i| i.size).sum();
    items.sort_by(|a, b| b.size.cmp(&a.size));
    (total_size, items)
}

/// Recursively searches for directories with `target_name` (e.g., "`node_modules`")
pub fn scan_recursive_for_target(
    root_path: &Path,
    target_name: &str,
    progress_cb: Option<&(dyn Fn() + Sync)>,
    allowlist: &Allowlist,
) -> Vec<ScannedItem> {
    let walker = WalkDir::new(root_path).skip_hidden(true).max_depth(5);

    let found_paths: Vec<PathBuf> = walker
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_dir() && e.file_name().to_string_lossy() == target_name)
        .map(|e| e.path())
        .filter(|p| !allowlist.is_allowed(p))
        .collect();

    let mut items: Vec<ScannedItem> = found_paths
        .par_iter()
        .map(|path| {
            if let Some(cb) = progress_cb {
                cb();
            }
            calculate_item_stats(path)
        })
        .collect();

    items.sort_by(|a, b| b.size.cmp(&a.size));
    items
}

pub fn calculate_item_stats(path: &Path) -> ScannedItem {
    let mut size = 0;
    let mut modified = SystemTime::UNIX_EPOCH;

    if let Ok(metadata) = fs::metadata(path)
        && let Ok(m) = metadata.modified()
    {
        modified = m;
    }

    // Use serial execution for individual item size calculation to avoid resource exhaustion
    for entry in WalkDir::new(path)
        .skip_hidden(false)
        .parallelism(jwalk::Parallelism::Serial)
        .into_iter()
        .flatten()
    {
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_file() {
                size += metadata.len();
            }
            if let Ok(m) = metadata.modified()
                && m > modified
            {
                modified = m;
            }
        }
    }

    ScannedItem {
        path: path.to_path_buf(),
        size,
        modified,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::allowlist::Allowlist;
    use anyhow::Result;
    use std::fs::File;
    use std::io::Write;
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    fn scan_path_structure() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path();

        let folder_a = root.join("FolderA");
        fs::create_dir(&folder_a)?;
        let mut f1 = File::create(folder_a.join("file1.txt"))?;
        f1.write_all(&[0u8; 100])?;

        thread::sleep(Duration::from_millis(100));

        let folder_b = root.join("FolderB");
        fs::create_dir(&folder_b)?;
        let mut f2 = File::create(folder_b.join("file2.txt"))?;
        f2.write_all(&[0u8; 200])?;

        let allowlist = Allowlist::new(vec![]);
        let (total_size, items) = scan_path(root, None, &allowlist);

        assert_eq!(total_size, 300);
        assert_eq!(items.len(), 2);
        Ok(())
    }

    #[test]
    fn scan_path_empty_dir() -> Result<()> {
        let dir = tempdir()?;
        let allowlist = Allowlist::new(vec![]);
        let (total_size, items) = scan_path(dir.path(), None, &allowlist);
        assert_eq!(total_size, 0);
        assert!(items.is_empty());
        Ok(())
    }

    #[test]
    fn scan_non_existent_path() {
        let path =
            PathBuf::from("/path/to/non/existent/directory/rust_mac_sweep_test_random_12345");
        let allowlist = Allowlist::new(vec![]);
        let (total_size, items) = scan_path(&path, None, &allowlist);
        assert_eq!(total_size, 0);
        assert!(items.is_empty());
    }

    #[test]
    fn scan_recursive_for_target_test() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path();

        let p1 = root.join("Project1");
        fs::create_dir(&p1)?;
        let nm1 = p1.join("node_modules");
        fs::create_dir(&nm1)?;
        let mut f1 = File::create(nm1.join("lib.js"))?;
        f1.write_all(&[0u8; 100])?;

        let p2 = root.join("Project2");
        fs::create_dir(&p2)?;
        let nm2 = p2.join("node_modules");
        fs::create_dir(&nm2)?;
        let mut f2 = File::create(nm2.join("index.js"))?;
        f2.write_all(&[0u8; 200])?;

        let allowlist = Allowlist::new(vec![]);
        let found_items = scan_recursive_for_target(root, "node_modules", None, &allowlist);

        assert_eq!(found_items.len(), 2);
        assert_eq!(found_items[0].size, 200);
        assert_eq!(found_items[1].size, 100);

        Ok(())
    }
}
