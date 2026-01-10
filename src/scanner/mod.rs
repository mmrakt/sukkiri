pub mod browsers;
pub mod dev;
pub mod docker;
pub mod trash;
pub mod user;
pub mod utils;
pub mod xcode;

use crate::allowlist::Allowlist;
use crate::constants::{DESKTOP_DIR, DOWNLOADS_DIR};
use crate::model::{CategoryType, ScanResult};
use crate::scanner::utils::scan_path;
use std::path::PathBuf;

pub fn scan_category(
    category: CategoryType,
    progress_cb: Option<&(dyn Fn() + Sync)>,
    allowlist: &Allowlist,
) -> ScanResult {
    let home = if let Ok(sudo_user) = std::env::var("SUDO_USER") {
        PathBuf::from("/Users").join(sudo_user)
    } else {
        dirs::home_dir().expect("Home directory not found")
    };

    let (items, description, root_path) = match category {
        CategoryType::XcodeJunk => xcode::scan_xcode_junk(&home, progress_cb, allowlist),
        CategoryType::SystemLogs => user::scan_system_logs(progress_cb, allowlist),
        CategoryType::SystemCache => {
            use crate::constants::SYSTEM_LIBRARY_CACHES;
            let path = PathBuf::from(SYSTEM_LIBRARY_CACHES);
            let (_, items) = scan_path(&path, progress_cb, allowlist);
            (items, "System cache files.", path)
        }
        CategoryType::UserLogs => user::scan_user_logs(&home, progress_cb, allowlist),
        CategoryType::BrowserCache => browsers::scan_browser_cache(&home, progress_cb, allowlist),
        CategoryType::UserCache => user::scan_user_cache(&home, progress_cb, allowlist),
        CategoryType::Downloads => {
            let path = home.join(DOWNLOADS_DIR);
            let (_, items) = scan_path(&path, progress_cb, allowlist);
            (items, "All files in Downloads folder.", path)
        }
        CategoryType::Trash => trash::scan_trash(&home, progress_cb, allowlist),
        CategoryType::DeveloperCaches => dev::scan_developer_caches(&home, progress_cb, allowlist),
        CategoryType::ScreenCapture => {
            let path = home.join(DESKTOP_DIR);
            let mut items = Vec::new();
            if path.exists() {
                let (_, dt_items) = scan_path(&path, progress_cb, allowlist);
                // Look for "Screenshot" or "スクリーンショット" prefix
                items.extend(dt_items.into_iter().filter(|i| {
                    let name = i.path.file_name().unwrap_or_default().to_string_lossy();
                    name.starts_with("Screenshot") || name.starts_with("スクリーンショット")
                }));
            }
            (items, "Screenshots on Desktop.", path)
        }
        CategoryType::NodeModules => dev::scan_node_modules(&home, progress_cb, allowlist),
        CategoryType::DockerImages => docker::scan_docker_unused_images(progress_cb, allowlist),
    };

    let total_size: u64 = items.iter().map(|i| i.size).sum();

    ScanResult {
        category,
        total_size,
        items,
        is_selected: false,
        description: description.to_string(),
        root_path,
    }
}
