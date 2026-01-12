use crate::allowlist::Allowlist;
use crate::model::{CategoryType, ScanResult, ScannedItem};
use crate::scanner::Scanner;
use std::path::PathBuf;
use std::process::Command;
use std::time::SystemTime;

pub struct DockerScanner;

impl Scanner for DockerScanner {
    fn category(&self) -> CategoryType {
        CategoryType::DockerImages
    }

    fn description(&self) -> String {
        "Unused Docker images (dangling=true)".to_string()
    }

    fn scan(&self, progress_cb: Option<&(dyn Fn() + Sync)>, allowlist: &Allowlist) -> ScanResult {
        // Docker scanning via CLI
        let items = scan_docker_unused_images_impl(progress_cb);

        let items: Vec<ScannedItem> = items
            .into_iter()
            .filter(|i| !allowlist.is_allowed(&i.path))
            .collect();

        let total_size = items.iter().map(|i| i.size).sum();
        let path = PathBuf::from("Docker"); // Virtual path

        ScanResult {
            category: self.category(),
            total_size,
            items,
            is_selected: false,
            description: self.description(),
            root_path: path,
        }
    }
}

fn scan_docker_unused_images_impl(progress_cb: Option<&(dyn Fn() + Sync)>) -> Vec<ScannedItem> {
    // Check if docker is available
    let check = Command::new("docker").arg("--version").output();
    if check.is_err() {
        return vec![];
    }

    // docker images -f "dangling=true" --format "{{.ID}}|{{.Size}}|{{.Repository}}:{{.Tag}}"
    let output = Command::new("docker")
        .args([
            "images",
            "-f",
            "dangling=true",
            "--format",
            "{{.ID}}|{{.Size}}|{{.Repository}}:{{.Tag}}",
        ])
        .output();

    let Ok(output) = output else {
        return vec![];
    };

    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut items = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 2 {
            let id = parts[0];
            let size_str = parts[1];
            let name = if parts.len() > 2 { parts[2] } else { "<none>" };

            let size = parse_docker_size(size_str);

            // Docker images don't have a real path, so we make a virtual one
            let path = PathBuf::from(format!("docker://{id}/{name}"));

            if let Some(cb) = progress_cb {
                cb();
            }

            items.push(ScannedItem {
                path,
                size,
                modified: SystemTime::now(),
            });
        }
    }

    items
}

fn parse_docker_size(size_str: &str) -> u64 {
    let s = size_str.trim().to_uppercase();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    if let Some(stripped) = s.strip_suffix("GB") {
        stripped
            .parse::<f64>()
            .map(|v| (v * 1_073_741_824.0) as u64)
            .unwrap_or(0)
    } else if let Some(stripped) = s.strip_suffix("MB") {
        stripped
            .parse::<f64>()
            .map(|v| (v * 1_048_576.0) as u64)
            .unwrap_or(0)
    } else if let Some(stripped) = s.strip_suffix("KB") {
        stripped
            .parse::<f64>()
            .map(|v| (v * 1_024.0) as u64)
            .unwrap_or(0)
    } else if let Some(stripped) = s.strip_suffix('B') {
        stripped.parse::<u64>().unwrap_or(0)
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_docker_size_test() {
        assert_eq!(parse_docker_size("1KB"), 1024);
        assert_eq!(parse_docker_size("1MB"), 1_048_576);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let expected = (1.5 * 1_073_741_824.0) as u64;
        assert_eq!(parse_docker_size("1.5GB"), expected);
        assert_eq!(parse_docker_size("500B"), 500);
        assert_eq!(parse_docker_size("0B"), 0);
    }
}
