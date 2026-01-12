use crate::allowlist::Allowlist;
use crate::constants::{
    BUN_CACHE, CARGO_REGISTRY, GO_MOD_CACHE, GRADLE_CACHE, NODE_MODULES, NPM_CACHE, PNPM_STORE,
    PROJECTS_DIR,
};
use crate::model::{CategoryType, ScanResult};
use crate::scanner::utils::scan_recursive_for_target;
use crate::scanner::{PathScanner, Scanner};
use std::path::{Path, PathBuf};

pub fn developer_caches_scanner(home: &Path) -> PathScanner {
    let targets = vec![
        home.join(NPM_CACHE),
        home.join(BUN_CACHE),
        home.join(PNPM_STORE),
        home.join(GO_MOD_CACHE),
        home.join(CARGO_REGISTRY),
        home.join(GRADLE_CACHE),
    ];

    let mut paths = Vec::new();
    for path in targets {
        if path.exists() {
            paths.push(path);
        }
    }

    PathScanner {
        category: CategoryType::DeveloperCaches,
        description: "Caches for npm, bun, pnpm, go, cargo, gradle, etc.".to_string(),
        paths,
    }
}

pub struct NodeModulesScanner {
    pub home: PathBuf,
}

impl Scanner for NodeModulesScanner {
    fn category(&self) -> CategoryType {
        CategoryType::NodeModules
    }

    fn description(&self) -> String {
        "Unused node_modules (Recursively found in ~/Projects)".to_string()
    }

    fn scan(&self, progress_cb: Option<&(dyn Fn() + Sync)>, allowlist: &Allowlist) -> ScanResult {
        let path = self.home.join(PROJECTS_DIR);
        let items = if path.exists() {
            scan_recursive_for_target(&path, NODE_MODULES, progress_cb, allowlist)
        } else {
            vec![]
        };

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
