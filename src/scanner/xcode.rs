use crate::allowlist::Allowlist;
use crate::constants::{CORE_SIMULATOR, XCODE_ARCHIVES, XCODE_DERIVED_DATA, XCODE_DEVICE_SUPPORT};
use crate::model::ScannedItem;
use crate::scanner::utils::scan_path;
use std::path::{Path, PathBuf};

pub fn scan_xcode_junk(
    home: &Path,
    progress_cb: Option<&(dyn Fn() + Sync)>,
    allowlist: &Allowlist,
) -> (Vec<ScannedItem>, &'static str, PathBuf) {
    let mut items = Vec::new();
    let mut paths = Vec::new();

    // DerivedData
    let derived_path = home.join(XCODE_DERIVED_DATA);
    if derived_path.exists() {
        let (_, mut derived_items) = scan_path(&derived_path, progress_cb, allowlist);
        items.append(&mut derived_items);
        paths.push(derived_path);
    }

    // Archives
    let archives_path = home.join(XCODE_ARCHIVES);
    if archives_path.exists() {
        let (_, mut archives_items) = scan_path(&archives_path, progress_cb, allowlist);
        items.append(&mut archives_items);
        paths.push(archives_path);
    }

    // iOS DeviceSupport
    let device_support_path = home.join(XCODE_DEVICE_SUPPORT);
    if device_support_path.exists() {
        let (_, mut ds_items) = scan_path(&device_support_path, progress_cb, allowlist);
        items.append(&mut ds_items);
        paths.push(device_support_path);
    }

    // CoreSimulator
    let core_sim_path = home.join(CORE_SIMULATOR);
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
