use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub struct Allowlist {
    rules: Vec<String>,
}

impl Allowlist {
    #[allow(dead_code)]
    pub fn new(rules: Vec<String>) -> Self {
        Self { rules }
    }
    /// Loads the allowlist from the default configuration path.
    /// Returns an empty allowlist if the file doesn't exist or errors.
    pub fn load() -> Self {
        let mut rules = Vec::new();

        if let Some(config_dir) = dirs::config_dir() {
            let allowlist_path = config_dir.join("sukkiri/allowlist.txt");
            if allowlist_path.exists()
                && let Ok(file) = fs::File::open(allowlist_path)
            {
                let reader = BufReader::new(file);
                for line in reader.lines().map_while(Result::ok) {
                    let trimmed = line.trim();
                    // Skip empty lines and comments
                    if !trimmed.is_empty() && !trimmed.starts_with('#') {
                        rules.push(trimmed.to_string());
                    }
                }
            }
        }

        Self { rules }
    }

    /// Checks if a path is allowed (should be ignored).
    /// Supports exact matches and simple prefix matches for directories.
    pub fn is_allowed(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        for rule in &self.rules {
            // Check for exact match or if path starts with rule (directory match)
            // Rules are treated as absolute paths or relative matching content?
            // PRD says "paths". Let's assume absolute paths or strict suffix/prefix?
            // Simple approach: string containment or starts_with if absolute.
            // If user puts "/Users/me/Secrets", we should ignore it.

            if path_str == *rule || path_str.starts_with(rule) {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_allowed() {
        let allowlist = Allowlist {
            rules: vec![
                "/Users/test/Secret".to_string(),
                "/Users/test/Projects/Keep".to_string(),
            ],
        };

        assert!(allowlist.is_allowed(Path::new("/Users/test/Secret")));
        assert!(allowlist.is_allowed(Path::new("/Users/test/Secret/file.txt"))); // Subfile
        assert!(allowlist.is_allowed(Path::new("/Users/test/Projects/Keep")));

        assert!(!allowlist.is_allowed(Path::new("/Users/test/Projects/DeleteMe")));
        assert!(!allowlist.is_allowed(Path::new("/Users/test/Public")));
    }
}
