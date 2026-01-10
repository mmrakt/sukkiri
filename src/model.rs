use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryType {
    SystemCache,
    UserLogs,
    XcodeDerivedData,
    NodeModules, // Placeholder
    #[allow(dead_code)]
    DockerImages, // Placeholder
}

impl CategoryType {
    pub fn name(&self) -> &str {
        match self {
            Self::SystemCache => "System Caches",
            Self::UserLogs => "User Logs",
            Self::XcodeDerivedData => "Xcode Junk",
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
