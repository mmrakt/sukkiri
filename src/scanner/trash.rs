use crate::constants::TRASH_DIR;
use crate::model::CategoryType;
use crate::scanner::PathScanner;
use std::path::Path;

pub fn trash_scanner(home: &Path) -> PathScanner {
    let path = home.join(TRASH_DIR);
    PathScanner {
        category: CategoryType::Trash,
        description: "Trash folder contents.".to_string(),
        paths: vec![path],
    }
}
