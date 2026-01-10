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
    let path = home.join("Library/Caches");
    let (_, mut items) = scan_path(&path, progress_cb, allowlist);

    // Filter out browser caches to avoid double counting
    items.retain(|item| {
        let p = &item.path;
        !p.to_string_lossy().contains("Google/Chrome")
            && !p.to_string_lossy().contains("com.apple.Safari")
            && !p.to_string_lossy().contains("Firefox")
    });

    (items, "User cache files (excluding browsers).", path)
}
