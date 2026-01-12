use crate::constants::{FIREFOX_CACHE, GOOGLE_CHROME_CACHE, SAFARI_CACHE};
use crate::model::CategoryType;
use crate::scanner::PathScanner;
use std::path::Path;

pub fn browser_cache_scanner(home: &Path) -> PathScanner {
    let mut paths = Vec::new();

    // Chrome
    let chrome_path = home.join(GOOGLE_CHROME_CACHE);
    if chrome_path.exists() {
        paths.push(chrome_path);
    }

    // Safari
    let safari_path = home.join(SAFARI_CACHE);
    if safari_path.exists() {
        paths.push(safari_path);
    }

    // Firefox
    let firefox_path = home.join(FIREFOX_CACHE);
    if firefox_path.exists() {
        paths.push(firefox_path);
    }

    PathScanner {
        category: CategoryType::BrowserCache,
        description: "Web browser caches (Chrome, Safari, Firefox).".to_string(),
        paths,
    }
}
