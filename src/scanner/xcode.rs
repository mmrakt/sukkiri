use crate::constants::{CORE_SIMULATOR, XCODE_ARCHIVES, XCODE_DERIVED_DATA, XCODE_DEVICE_SUPPORT};
use crate::model::CategoryType;
use crate::scanner::PathScanner;
use std::path::Path;

pub fn xcode_scanner(home: &Path) -> PathScanner {
    let mut paths = Vec::new();

    // DerivedData
    let derived_path = home.join(XCODE_DERIVED_DATA);
    if derived_path.exists() {
        paths.push(derived_path);
    }

    // Archives
    let archives_path = home.join(XCODE_ARCHIVES);
    if archives_path.exists() {
        paths.push(archives_path);
    }

    // iOS DeviceSupport
    let device_support_path = home.join(XCODE_DEVICE_SUPPORT);
    if device_support_path.exists() {
        paths.push(device_support_path);
    }

    // CoreSimulator
    let core_sim_path = home.join(CORE_SIMULATOR);
    if core_sim_path.exists() {
        paths.push(core_sim_path);
    }

    PathScanner {
        category: CategoryType::XcodeJunk,
        description: "Xcode build artifacts, archives, and device support.".to_string(),
        paths,
    }
}
