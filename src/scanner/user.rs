use crate::allowlist::Allowlist;
use crate::constants::{
    FIREFOX_CACHE, GOOGLE_CHROME_CACHE, LIBRARY_CACHES, LIBRARY_LOGS, SAFARI_CACHE,
    SYSTEM_LIBRARY_LOGS, VAR_LOG,
};
use crate::model::{CategoryType, ScanResult, ScannedItem};
use crate::scanner::utils::scan_path;
use crate::scanner::{PathScanner, Scanner};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

pub fn system_logs_scanner() -> PathScanner {
    let mut paths = Vec::new();
    let path = PathBuf::from(SYSTEM_LIBRARY_LOGS);
    if path.exists() {
        paths.push(path);
    }
    let var_log = PathBuf::from(VAR_LOG);
    if var_log.exists() {
        paths.push(var_log);
    }

    PathScanner {
        category: CategoryType::SystemLogs,
        description: "System log files (/Library/Logs, /private/var/log).".to_string(),
        paths,
    }
}

pub fn user_logs_scanner(home: &Path) -> PathScanner {
    let path = home.join(LIBRARY_LOGS);
    PathScanner {
        category: CategoryType::UserLogs,
        description: "User log files.".to_string(),
        paths: vec![path],
    }
}

pub struct UserCacheScanner {
    pub home: PathBuf,
}

impl Scanner for UserCacheScanner {
    fn category(&self) -> CategoryType {
        CategoryType::UserCache
    }

    fn description(&self) -> String {
        "User cache files (including sandboxed apps).".to_string()
    }

    fn scan(&self, progress_cb: Option<&(dyn Fn() + Sync)>, allowlist: &Allowlist) -> ScanResult {
        let path = self.home.join(LIBRARY_CACHES);
        let (_, mut items) = scan_path(&path, progress_cb, allowlist);

        // Filter out standard browser caches from standard user cache
        items.retain(|item| {
            let p = &item.path;
            !p.to_string_lossy().contains(GOOGLE_CHROME_CACHE)
                && !p.to_string_lossy().contains(SAFARI_CACHE)
                && !p.to_string_lossy().contains(FIREFOX_CACHE)
        });

        // Scan ~/Library/Containers/*/Data/Library/Caches
        let containers_path = self.home.join("Library/Containers");
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

        ScanResult {
            category: self.category(),
            total_size: items.iter().map(|i| i.size).sum(),
            items,
            is_selected: false,
            description: self.description(),
            root_path: path,
        }
    }
}

pub struct ScreenCaptureScanner {
    pub home: PathBuf,
}

impl Scanner for ScreenCaptureScanner {
    fn category(&self) -> CategoryType {
        CategoryType::ScreenCapture
    }

    fn description(&self) -> String {
        "Screenshots on Desktop.".to_string()
    }

    fn scan(&self, progress_cb: Option<&(dyn Fn() + Sync)>, allowlist: &Allowlist) -> ScanResult {
        use crate::constants::DESKTOP_DIR;
        let path = self.home.join(DESKTOP_DIR);
        let mut items = Vec::new();

        if path.exists() {
            let (_, dt_items) = scan_path(&path, progress_cb, allowlist);
            // Look for "Screenshot" or "スクリーンショット" prefix
            items.extend(dt_items.into_iter().filter(|i| {
                let name = i.path.file_name().unwrap_or_default().to_string_lossy();
                name.starts_with("Screenshot") || name.starts_with("スクリーンショット")
            }));
        }

        ScanResult {
            category: self.category(),
            total_size: items.iter().map(|i| i.size).sum(),
            items,
            is_selected: false,
            description: self.description(),
            root_path: path,
        }
    }
}
