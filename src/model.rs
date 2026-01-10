use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryType {
    XcodeJunk,
    SystemLogs,
    SystemCache,
    UserLogs,
    UserCache,
    BrowserCache,
    Downloads,
    Trash,
    DeveloperCaches,
    ScreenCapture,
    NodeModules,
    #[allow(dead_code)]
    DockerImages,
}

impl CategoryType {
    pub fn name(&self) -> &str {
        match self {
            Self::XcodeJunk => "Xcode Junk",
            Self::SystemLogs => "System Log Files",
            Self::SystemCache => "System Cache Files",
            Self::UserLogs => "User Log Files",
            Self::UserCache => "User Cache Files",
            Self::BrowserCache => "Browser Cache",
            Self::Downloads => "Downloads",
            Self::Trash => "Trash",
            Self::DeveloperCaches => "Developer Caches",
            Self::ScreenCapture => "Screen Capture Files",
            Self::NodeModules => "Node Modules",
            Self::DockerImages => "Docker Images",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScannedItem {
    pub path: PathBuf,
    pub size: u64,
    #[allow(dead_code)]
    pub modified: SystemTime,
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub category: CategoryType,
    pub total_size: u64,
    pub items: Vec<ScannedItem>,
    pub is_selected: bool,
    pub description: String,
    pub root_path: PathBuf,
}
