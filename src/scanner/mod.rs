pub mod browsers;
pub mod dev;
pub mod docker;
pub mod trash;
pub mod user;
pub mod utils;
pub mod xcode;

use crate::allowlist::Allowlist;
use crate::model::{CategoryType, ScanResult};
use crate::scanner::utils::scan_path;
use std::path::PathBuf;

pub trait Scanner: Send + Sync {
    fn category(&self) -> CategoryType;
    fn description(&self) -> String;
    fn scan(&self, progress_cb: Option<&(dyn Fn() + Sync)>, allowlist: &Allowlist) -> ScanResult;
}

pub struct PathScanner {
    pub category: CategoryType,
    pub description: String,
    pub paths: Vec<PathBuf>,
}

impl Scanner for PathScanner {
    fn category(&self) -> CategoryType {
        self.category
    }

    fn description(&self) -> String {
        self.description.clone()
    }

    fn scan(&self, progress_cb: Option<&(dyn Fn() + Sync)>, allowlist: &Allowlist) -> ScanResult {
        let mut all_items = Vec::new();

        for path in &self.paths {
            let (_, mut items) = scan_path(path, progress_cb, allowlist);
            all_items.append(&mut items);
        }

        let total_size: u64 = all_items.iter().map(|i| i.size).sum();

        // Determine a "root" path for display.
        // If there is only one path, use it. If multiple, maybe just the parent dir of the first or common path?
        // OR just the first path.
        let root_path = if self.paths.is_empty() {
            dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
        } else {
            self.paths[0].clone()
        };

        ScanResult {
            category: self.category,
            total_size,
            items: all_items,
            is_selected: false,
            description: self.description.clone(),
            root_path,
        }
    }
}

pub fn get_all_scanners() -> Vec<Box<dyn Scanner>> {
    let home = dirs::home_dir().expect("Home directory not found");

    vec![
        // Xcode: DerivedData, Archives, DeviceSupport
        Box::new(xcode::xcode_scanner(&home)),
        // System Logs: /Library/Logs, /private/var/log
        Box::new(user::system_logs_scanner()),
        // System Cache: /Library/Caches
        Box::new(PathScanner {
            category: CategoryType::SystemCache,
            description: "System cache files.".to_string(),
            paths: vec![PathBuf::from(crate::constants::SYSTEM_LIBRARY_CACHES)],
        }),
        // User Logs: ~/Library/Logs
        Box::new(user::user_logs_scanner(&home)),
        // User Cache: ~/Library/Caches (filtered) + Containers
        Box::new(user::UserCacheScanner { home: home.clone() }),
        // Browser Cache: Chrome, Safari, Firefox
        Box::new(browsers::browser_cache_scanner(&home)),
        // Downloads: ~/Downloads
        Box::new(PathScanner {
            category: CategoryType::Downloads,
            description: "All files in Downloads folder.".to_string(),
            paths: vec![home.join(crate::constants::DOWNLOADS_DIR)],
        }),
        // Trash: ~/.Trash
        Box::new(trash::trash_scanner(&home)),
        // Developer Caches: .npm, .cargo, etc.
        Box::new(dev::developer_caches_scanner(&home)),
        // Screen Capture: Desktop screenshots
        Box::new(user::ScreenCaptureScanner { home: home.clone() }),
        // Node Modules: Recursive search in ~/Projects
        Box::new(dev::NodeModulesScanner { home: home.clone() }),
        // Docker: dangling images
        Box::new(docker::DockerScanner),
    ]
}
