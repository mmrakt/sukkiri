use crate::allowlist::Allowlist;
use crate::constants::TRASH_DIR;
use crate::model::ScannedItem;
use crate::scanner::utils::scan_path;
use std::path::{Path, PathBuf};

pub fn scan_trash(
    home: &Path,
    progress_cb: Option<&(dyn Fn() + Sync)>,
    allowlist: &Allowlist,
) -> (Vec<ScannedItem>, &'static str, PathBuf) {
    let path = home.join(TRASH_DIR);
    let (_, items) = scan_path(&path, progress_cb, allowlist);
    (items, "Trash folder contents.", path)
}
