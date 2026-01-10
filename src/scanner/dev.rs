use crate::allowlist::Allowlist;
use crate::constants::{
    BUN_CACHE, CARGO_REGISTRY, GO_MOD_CACHE, GRADLE_CACHE, NPM_CACHE, PNPM_STORE,
};
use crate::model::ScannedItem;
use crate::scanner::utils::scan_path;
use std::path::{Path, PathBuf};

pub fn scan_developer_caches(
    home: &Path,
    progress_cb: Option<&(dyn Fn() + Sync)>,
    allowlist: &Allowlist,
) -> (Vec<ScannedItem>, &'static str, PathBuf) {
    let mut items = Vec::new();
    let mut paths = Vec::new();

    let targets = vec![
        home.join(NPM_CACHE),
        home.join(BUN_CACHE),
        home.join(PNPM_STORE),
        home.join(GO_MOD_CACHE),
        home.join(CARGO_REGISTRY),
        home.join(GRADLE_CACHE),
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

pub fn scan_node_modules(
    home: &Path,
    progress_cb: Option<&(dyn Fn() + Sync)>,
    allowlist: &Allowlist,
) -> (Vec<ScannedItem>, &'static str, PathBuf) {
    use crate::constants::{NODE_MODULES, PROJECTS_DIR};
    use crate::scanner::utils::scan_recursive_for_target;

    let path = home.join(PROJECTS_DIR);
    let items = if path.exists() {
        scan_recursive_for_target(&path, NODE_MODULES, progress_cb, allowlist)
    } else {
        vec![]
    };
    (
        items,
        "Unused node_modules (Recursively found in ~/Projects)",
        path,
    )
}
