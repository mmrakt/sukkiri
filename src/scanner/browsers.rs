use crate::allowlist::Allowlist;
use crate::constants::{FIREFOX_CACHE, GOOGLE_CHROME_CACHE, LIBRARY_CACHES, SAFARI_CACHE};
use crate::model::ScannedItem;
use crate::scanner::utils::scan_path;
use std::path::{Path, PathBuf};

pub fn scan_browser_cache(
    home: &Path,
    progress_cb: Option<&(dyn Fn() + Sync)>,
    allowlist: &Allowlist,
) -> (Vec<ScannedItem>, &'static str, PathBuf) {
    let mut items = Vec::new();
    let mut paths = Vec::new();

    // Chrome
    let chrome_path = home.join(GOOGLE_CHROME_CACHE);
    if chrome_path.exists() {
        let (_, mut chrome_items) = scan_path(&chrome_path, progress_cb, allowlist);
        items.append(&mut chrome_items);
        paths.push(chrome_path);
    }

    // Safari
    let safari_path = home.join(SAFARI_CACHE);
    if safari_path.exists() {
        let (_, mut safari_items) = scan_path(&safari_path, progress_cb, allowlist);
        items.append(&mut safari_items);
        paths.push(safari_path);
    }

    // Firefox
    let firefox_path = home.join(FIREFOX_CACHE);
    if firefox_path.exists() {
        let (_, mut firefox_items) = scan_path(&firefox_path, progress_cb, allowlist);
        items.append(&mut firefox_items);
        paths.push(firefox_path);
    }

    let root = if paths.is_empty() {
        home.join(LIBRARY_CACHES)
    } else {
        paths[0].clone()
    };

    (items, "Web browser caches (Chrome, Safari, Firefox).", root)
}
