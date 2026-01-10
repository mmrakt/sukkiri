use crate::allowlist::Allowlist;
use crate::constants::{
    FIREFOX_CACHE, GOOGLE_CHROME_CACHE, LIBRARY_CACHES, LIBRARY_LOGS, SAFARI_CACHE,
    SYSTEM_LIBRARY_LOGS, VAR_LOG,
};
use crate::model::ScannedItem;
use crate::scanner::utils::scan_path;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

pub fn scan_system_logs(
    progress_cb: Option<&(dyn Fn() + Sync)>,
    allowlist: &Allowlist,
) -> (Vec<ScannedItem>, &'static str, PathBuf) {
    let mut items = Vec::new();
    let mut paths = Vec::new();

    let path = PathBuf::from(SYSTEM_LIBRARY_LOGS);
    if path.exists() {
        let (_, mut log_items) = scan_path(&path, progress_cb, allowlist);
        items.append(&mut log_items);
        paths.push(path);
    }

    // Expanded: /private/var/log
    let var_log = PathBuf::from(VAR_LOG);
    if var_log.exists() {
        let (_, mut var_items) = scan_path(&var_log, progress_cb, allowlist);
        items.append(&mut var_items);
        paths.push(var_log);
    }

    let root = if paths.is_empty() {
        PathBuf::from(SYSTEM_LIBRARY_LOGS)
    } else {
        paths[0].clone()
    };

    (
        items,
        "System log files (/Library/Logs, /private/var/log).",
        root,
    )
}

pub fn scan_user_logs(
    home: &Path,
    progress_cb: Option<&(dyn Fn() + Sync)>,
    allowlist: &Allowlist,
) -> (Vec<ScannedItem>, &'static str, PathBuf) {
    let path = home.join(LIBRARY_LOGS);
    let (_, items) = scan_path(&path, progress_cb, allowlist);
    (items, "User log files.", path)
}

pub fn scan_user_cache(
    home: &Path,
    progress_cb: Option<&(dyn Fn() + Sync)>,
    allowlist: &Allowlist,
) -> (Vec<ScannedItem>, &'static str, PathBuf) {
    // Standard User Cache
    let path = home.join(LIBRARY_CACHES);
    let (_, mut items) = scan_path(&path, progress_cb, allowlist);

    // Filter out standard browser caches from standard user cache
    items.retain(|item| {
        let p = &item.path;
        !p.to_string_lossy().contains(GOOGLE_CHROME_CACHE)
            && !p.to_string_lossy().contains(SAFARI_CACHE)
            && !p.to_string_lossy().contains(FIREFOX_CACHE)
    });

    // Scan ~/Library/Containers/*/Data/Library/Caches
    let containers_path = home.join("Library/Containers");
    if containers_path.exists()
        && let Ok(entries) = fs::read_dir(&containers_path)
    {
        let container_caches: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|e| e.path().join("Data/Library/Caches"))
            .filter(|p| p.exists())
            .collect();

        // Scan each found container cache
        // Parallelize scanning across different path roots
        let container_items: Vec<ScannedItem> = container_caches
            .par_iter()
            .flat_map(|path| {
                let (_, items) = scan_path(path, progress_cb, allowlist);
                items
            })
            .collect();

        items.extend(container_items);
    }

    (items, "User cache files (including sandboxed apps).", path)
}
