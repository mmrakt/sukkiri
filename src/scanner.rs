use crate::allowlist::Allowlist; // Added
use crate::model::{CategoryType, ScanResult, ScannedItem};
use jwalk::WalkDir;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub fn scan_category(
    category: CategoryType,
    progress_cb: Option<&(dyn Fn() + Sync)>,
    allowlist: &Allowlist, // Added
) -> ScanResult {
    let home = if let Ok(sudo_user) = std::env::var("SUDO_USER") {
        PathBuf::from("/Users").join(sudo_user)
    } else {
        dirs::home_dir().expect("Home directory not found")
    };

    let (items, description, root_path) = match category {
        CategoryType::XcodeJunk => scan_xcode_junk(&home, progress_cb, allowlist),
        CategoryType::SystemLogs => {
            let mut items = Vec::new();
            let mut paths = Vec::new();

            let path = PathBuf::from("/Library/Logs");
            if path.exists() {
                let (_, mut log_items) = scan_path(&path, progress_cb, allowlist);
                items.append(&mut log_items);
                paths.push(path);
            }

            // Expanded: /private/var/log
            let var_log = PathBuf::from("/private/var/log");
            if var_log.exists() {
                let (_, mut var_items) = scan_path(&var_log, progress_cb, allowlist);
                items.append(&mut var_items);
                paths.push(var_log);
            }

            let root = if paths.is_empty() {
                PathBuf::from("/Library/Logs")
            } else {
                paths[0].clone()
            };

            (
                items,
                "System log files (/Library/Logs, /private/var/log).",
                root,
            )
        }
        CategoryType::SystemCache => {
            let path = PathBuf::from("/Library/Caches");
            let (_, items) = scan_path(&path, progress_cb, allowlist);
            (items, "System cache files.", path)
        }
        CategoryType::UserLogs => {
            let path = home.join("Library/Logs");
            let (_, items) = scan_path(&path, progress_cb, allowlist);
            (items, "User log files.", path)
        }
        CategoryType::BrowserCache => scan_browser_cache(&home, progress_cb, allowlist),
        CategoryType::UserCache => scan_user_cache(&home, progress_cb, allowlist),
        CategoryType::Downloads => {
            let path = home.join("Downloads");
            let (_, items) = scan_path(&path, progress_cb, allowlist);
            (items, "All files in Downloads folder.", path)
        }
        CategoryType::Trash => scan_trash(&home, progress_cb, allowlist),
        CategoryType::DeveloperCaches => scan_developer_caches(&home, progress_cb, allowlist),
        CategoryType::ScreenCapture => {
            let path = home.join("Desktop");
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
        CategoryType::NodeModules => {
            let path = home.join("Projects");
            let items = if path.exists() {
                scan_recursive_for_target(&path, "node_modules", progress_cb, allowlist)
            } else {
                vec![]
            };
            (
                items,
                "Unused node_modules (Recursively found in ~/Projects)",
                path,
            )
        }
        CategoryType::DockerImages => {
            // Docker scanning via CLI
            // Allowlist doesn't apply to Docker images easily as they are virtual paths for now.
            // But we can check if allowlist rule matches "docker://..."
            let items = scan_docker_unused_images(progress_cb);
            // Filter docker items?
            let items: Vec<ScannedItem> = items
                .into_iter()
                .filter(|i| !allowlist.is_allowed(&i.path))
                .collect();

            let path = PathBuf::from("Docker"); // Virtual path
            (items, "Unused Docker images (dangling=true)", path)
        }
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

fn scan_browser_cache(
    home: &Path,
    progress_cb: Option<&(dyn Fn() + Sync)>,
    allowlist: &Allowlist,
) -> (Vec<ScannedItem>, &'static str, PathBuf) {
    let mut items = Vec::new();
    let mut paths = Vec::new();

    // Chrome
    let chrome_path = home.join("Library/Caches/Google/Chrome");
    if chrome_path.exists() {
        let (_, mut chrome_items) = scan_path(&chrome_path, progress_cb, allowlist);
        items.append(&mut chrome_items);
        paths.push(chrome_path);
    }

    // Safari
    let safari_path = home.join("Library/Caches/com.apple.Safari");
    if safari_path.exists() {
        let (_, mut safari_items) = scan_path(&safari_path, progress_cb, allowlist);
        items.append(&mut safari_items);
        paths.push(safari_path);
    }

    // Firefox
    let firefox_path = home.join("Library/Caches/Firefox");
    if firefox_path.exists() {
        let (_, mut firefox_items) = scan_path(&firefox_path, progress_cb, allowlist);
        items.append(&mut firefox_items);
        paths.push(firefox_path);
    }

    let root = if paths.is_empty() {
        home.join("Library/Caches")
    } else {
        paths[0].clone()
    };

    (items, "Web browser caches (Chrome, Safari, Firefox).", root)
}

fn scan_user_cache(
    home: &Path,
    progress_cb: Option<&(dyn Fn() + Sync)>,
    allowlist: &Allowlist,
) -> (Vec<ScannedItem>, &'static str, PathBuf) {
    // Standard User Cache
    let path = home.join("Library/Caches");
    let (_, mut items) = scan_path(&path, progress_cb, allowlist);

    // Filter out standard browser caches from standard user cache
    items.retain(|item| {
        let p = &item.path;
        !p.to_string_lossy().contains("Google/Chrome")
            && !p.to_string_lossy().contains("com.apple.Safari")
            && !p.to_string_lossy().contains("Firefox")
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

fn scan_xcode_junk(
    home: &Path,
    progress_cb: Option<&(dyn Fn() + Sync)>,
    allowlist: &Allowlist,
) -> (Vec<ScannedItem>, &'static str, PathBuf) {
    let mut items = Vec::new();
    let mut paths = Vec::new();

    // DerivedData
    let derived_path = home.join("Library/Developer/Xcode/DerivedData");
    if derived_path.exists() {
        let (_, mut derived_items) = scan_path(&derived_path, progress_cb, allowlist);
        items.append(&mut derived_items);
        paths.push(derived_path);
    }

    // Archives
    let archives_path = home.join("Library/Developer/Xcode/Archives");
    if archives_path.exists() {
        let (_, mut archives_items) = scan_path(&archives_path, progress_cb, allowlist);
        items.append(&mut archives_items);
        paths.push(archives_path);
    }

    // iOS DeviceSupport
    let device_support_path = home.join("Library/Developer/Xcode/iOS DeviceSupport");
    if device_support_path.exists() {
        let (_, mut ds_items) = scan_path(&device_support_path, progress_cb, allowlist);
        items.append(&mut ds_items);
        paths.push(device_support_path);
    }

    // CoreSimulator (Expanded based on feedback)
    let core_sim_path = home.join("Library/Developer/CoreSimulator");
    if core_sim_path.exists() {
        let (_, mut sim_items) = scan_path(&core_sim_path, progress_cb, allowlist);
        items.append(&mut sim_items);
        paths.push(core_sim_path);
    }

    let root = if paths.is_empty() {
        home.join("Library/Developer/Xcode")
    } else {
        paths[0].clone()
    };

    (
        items,
        "Xcode build artifacts, archives, and device support.",
        root,
    )
}

fn scan_trash(
    home: &Path,
    progress_cb: Option<&(dyn Fn() + Sync)>,
    allowlist: &Allowlist,
) -> (Vec<ScannedItem>, &'static str, PathBuf) {
    let path = home.join(".Trash");
    let (_, items) = scan_path(&path, progress_cb, allowlist);
    (items, "Trash folder contents.", path)
}

fn scan_developer_caches(
    home: &Path,
    progress_cb: Option<&(dyn Fn() + Sync)>,
    allowlist: &Allowlist,
) -> (Vec<ScannedItem>, &'static str, PathBuf) {
    let mut items = Vec::new();
    let mut paths = Vec::new();

    // npm: ~/.npm
    // bun: ~/.bun/install/cache
    // pnpm: ~/.pnpm-store
    // go: ~/go/pkg/mod
    // pip: ~/Library/Caches/pip (Already covered by UserCache, but worth checking explicitly if we want to isolate)
    // cargo: ~/.cargo/registry (Optional, risky?) -> User asked for "Developer tool related caches". Registry is cache.

    let targets = vec![
        home.join(".npm"),
        home.join(".bun/install/cache"),
        home.join(".pnpm-store"),
        home.join("go/pkg/mod"),
        home.join(".cargo/registry"),
        home.join(".gradle/caches"),
    ];

    for path in targets {
        if path.exists() {
            let (_, mut sub_items) = scan_path(&path, progress_cb, allowlist);
            items.append(&mut sub_items);
            paths.push(path);
        }
    }

    // Default root path logic
    let root = home.to_path_buf(); // Fallback
    (
        items,
        "Caches for npm, bun, pnpm, go, cargo, gradle, etc.",
        root,
    )
}

/// Helper function to scan a path and return total size and items.
pub fn scan_path(
    target_path: &Path,
    progress_cb: Option<&(dyn Fn() + Sync)>,
    allowlist: &Allowlist, // Added
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
        .filter(|path| !allowlist.is_allowed(path)) // Added check
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
    allowlist: &Allowlist, // Added
) -> Vec<ScannedItem> {
    let walker = WalkDir::new(root_path).skip_hidden(true).max_depth(5);

    let found_paths: Vec<PathBuf> = walker
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_dir() && e.file_name().to_string_lossy() == target_name)
        .map(|e| e.path())
        .filter(|p| !allowlist.is_allowed(p)) // Added check
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

/// Scans for unused (dangling) Docker images using CLI
pub fn scan_docker_unused_images(progress_cb: Option<&(dyn Fn() + Sync)>) -> Vec<ScannedItem> {
    use std::process::Command;

    // Check if docker is available
    let check = Command::new("docker").arg("--version").output();
    if check.is_err() {
        return vec![];
    }

    // docker images -f "dangling=true" --format "{{.ID}}|{{.Size}}|{{.Repository}}:{{.Tag}}"
    let output = Command::new("docker")
        .args([
            "images",
            "-f",
            "dangling=true",
            "--format",
            "{{.ID}}|{{.Size}}|{{.Repository}}:{{.Tag}}",
        ])
        .output();

    let Ok(output) = output else {
        return vec![];
    };

    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut items = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 2 {
            let id = parts[0];
            let size_str = parts[1];
            let name = if parts.len() > 2 { parts[2] } else { "<none>" };

            let size = parse_docker_size(size_str);

            // Docker images don't have a real path, so we make a virtual one
            let path = PathBuf::from(format!("docker://{id}/{name}"));

            if let Some(cb) = progress_cb {
                cb();
            }

            items.push(ScannedItem {
                path,
                size,
                modified: SystemTime::now(),
            });
        }
    }

    items
}

fn parse_docker_size(size_str: &str) -> u64 {
    let s = size_str.trim().to_uppercase();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    if let Some(stripped) = s.strip_suffix("GB") {
        stripped
            .parse::<f64>()
            .map(|v| (v * 1_073_741_824.0) as u64)
            .unwrap_or(0)
    } else if let Some(stripped) = s.strip_suffix("MB") {
        stripped
            .parse::<f64>()
            .map(|v| (v * 1_048_576.0) as u64)
            .unwrap_or(0)
    } else if let Some(stripped) = s.strip_suffix("KB") {
        stripped
            .parse::<f64>()
            .map(|v| (v * 1_024.0) as u64)
            .unwrap_or(0)
    } else if let Some(stripped) = s.strip_suffix('B') {
        stripped.parse::<u64>().unwrap_or(0)
    } else {
        0
    }
}

fn calculate_item_stats(path: &Path) -> ScannedItem {
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

    #[test]
    fn parse_docker_size_test() {
        assert_eq!(parse_docker_size("1KB"), 1024);
        assert_eq!(parse_docker_size("1MB"), 1_048_576);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let expected = (1.5 * 1_073_741_824.0) as u64;
        assert_eq!(parse_docker_size("1.5GB"), expected);
        assert_eq!(parse_docker_size("500B"), 500);
        assert_eq!(parse_docker_size("0B"), 0);
    }
}
