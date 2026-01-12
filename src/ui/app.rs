use crate::allowlist::Allowlist;
use crate::cleaner;
use crate::model::ScanResult;
use crate::model::{CategoryType, ScanProgress};
use crate::scanner;
use anyhow::Result;
use humansize::{BINARY, format_size};
use ratatui::widgets::ListState;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use sysinfo::Disks;

pub enum AppState {
    Browsing,
    Confirming,
    Cleaning,
    Scanning,     // New state for scanning
    Done(String), // Done message
}

pub struct App {
    pub results: Vec<ScanResult>,
    pub list_state: ListState,
    pub state: AppState,
    pub disks: Disks,
    // Channel receiver for cleaning thread results
    pub cleaning_rx: Option<mpsc::Receiver<Result<String, String>>>,
    // Scanning
    pub scan_rx: Option<mpsc::Receiver<ScanUpdate>>,
    pub scan_progress: HashMap<CategoryType, ScanProgress>,
    pub total_categories: usize,
}

pub enum ScanUpdate {
    Progress(ScanProgress),
    Result(ScanResult),
}

impl App {
    pub fn new_scanning() -> Self {
        let disks = Disks::new_with_refreshed_list();
        Self {
            results: Vec::new(),
            list_state: ListState::default(),
            state: AppState::Scanning,
            disks,
            cleaning_rx: None,
            scan_rx: None,
            scan_progress: HashMap::new(),
            total_categories: 0,
        }
    }

    pub fn next(&mut self) {
        if self.results.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.results.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.results.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.results.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn toggle(&mut self) {
        if let Some(i) = self.list_state.selected()
            && i < self.results.len()
        {
            self.results[i].is_selected = !self.results[i].is_selected;
        }
    }

    pub fn total_selected_size(&self) -> u64 {
        self.results
            .iter()
            .filter(|r| r.is_selected)
            .map(|r| r.total_size)
            .sum()
    }

    pub fn clean_selected(&mut self) {
        // Collect all items to delete
        let mut items_to_delete = Vec::new();
        for result in &self.results {
            if result.is_selected {
                items_to_delete.extend(result.items.clone());
            }
        }

        if items_to_delete.is_empty() {
            self.state = AppState::Done("Nothing selected to clean.".to_string());
            return;
        }

        self.state = AppState::Cleaning;

        // Threaded cleaning
        let (tx, rx) = mpsc::channel();
        self.cleaning_rx = Some(rx);

        // Move items to a separate thread
        let items = items_to_delete;
        thread::spawn(move || {
            // Artificial delay to make "Cleaning" state visible if it's too fast?
            // thread::sleep(Duration::from_millis(500));

            let size = items.iter().map(|i| i.size).sum::<u64>();
            match cleaner::move_to_trash(&items) {
                Ok(()) => {
                    let msg = format!("Successfully cleaned {}!", format_size(size, BINARY));
                    let _ = tx.send(Ok(msg));
                }
                Err(e) => {
                    let _ = tx.send(Err(format!("Error during cleaning: {e}")));
                }
            }
        });
    }

    pub fn check_cleaning_status(&mut self) {
        if let Some(rx) = &self.cleaning_rx
            && let Ok(result) = rx.try_recv()
        {
            match result {
                Ok(msg) => {
                    self.state = AppState::Done(msg);
                    // Clear selection and items (naive update)
                    for result in &mut self.results {
                        if result.is_selected {
                            result.is_selected = false;
                            result.items.clear();
                            result.total_size = 0;
                        }
                    }

                    // Refresh disk info after cleaning
                    self.disks.refresh(true);
                }
                Err(err_msg) => {
                    self.state = AppState::Done(err_msg);
                }
            }
            self.cleaning_rx = None; // Detach receiver
        }
    }
    pub fn start_scan(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.scan_rx = Some(rx);

        let allowlist = Arc::new(Allowlist::load());

        let categories = vec![
            CategoryType::XcodeJunk,
            CategoryType::SystemLogs,
            CategoryType::SystemCache,
            CategoryType::UserLogs,
            CategoryType::UserCache,
            CategoryType::BrowserCache,
            CategoryType::Downloads,
            CategoryType::Trash,
            CategoryType::DeveloperCaches,
            CategoryType::ScreenCapture,
            CategoryType::NodeModules,
            CategoryType::DockerImages,
        ];
        self.total_categories = categories.len();

        for category in categories {
            // Initialize progress for this category
            self.scan_progress.insert(
                category,
                ScanProgress {
                    category,
                    items_count: 0,
                    status: "Waiting...".to_string(),
                },
            );

            let tx_clone = tx.clone();
            let allowlist_clone = Arc::clone(&allowlist);

            thread::spawn(move || {
                let cat_name = category; // copy

                // Progress callback
                let tx_progress = tx_clone.clone();
                let cb = move || {
                    let _ = tx_progress.send(ScanUpdate::Progress(ScanProgress {
                        category: cat_name,
                        items_count: 1, // This will need to be accumulated in the main thread
                        status: "Scanning...".to_string(),
                    }));
                };

                // Perform scan
                let res = scanner::scan_category(category, Some(&cb), &allowlist_clone);

                let _ = tx_clone.send(ScanUpdate::Result(res));
            });
        }
    }

    pub fn check_scan_status(&mut self) {
        if let Some(rx) = &self.scan_rx {
            // Non-blocking check for all available messages
            while let Ok(update) = rx.try_recv() {
                match update {
                    ScanUpdate::Progress(progress) => {
                        if let Some(entry) = self.scan_progress.get_mut(&progress.category) {
                            entry.items_count += progress.items_count; // Aggregate counts
                            entry.status = progress.status;
                        }
                    }
                    ScanUpdate::Result(result) => {
                        if let Some(entry) = self.scan_progress.get_mut(&result.category) {
                            entry.status = "Done".to_string();
                        }
                        self.results.push(result);
                    }
                }
            }

            // Check if scanning is complete
            if self.results.len() == self.total_categories {
                self.results.sort_by(|a, b| b.total_size.cmp(&a.total_size));

                if !self.results.is_empty() {
                    self.list_state.select(Some(0));
                }
                self.state = AppState::Browsing;
                self.scan_rx = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_check_scan_status_updates() {
        let mut app = App::new_scanning();

        // Setup manual test state
        let category = CategoryType::XcodeJunk;
        app.scan_progress.insert(
            category,
            ScanProgress {
                category,
                items_count: 0,
                status: "Waiting...".to_string(),
            },
        );
        app.total_categories = 1;

        // Create a channel to simulate scan updates
        let (tx, rx) = mpsc::channel();
        app.scan_rx = Some(rx);

        // 1. Send Progress Update
        tx.send(ScanUpdate::Progress(ScanProgress {
            category,
            items_count: 5,
            status: "Scanning...".to_string(),
        }))
        .unwrap();

        // Process update
        app.check_scan_status();

        // Verify progress
        let progress = app
            .scan_progress
            .get(&category)
            .expect("Category should exist");
        assert_eq!(progress.items_count, 5);
        assert_eq!(progress.status, "Scanning...");
        assert!(matches!(app.state, AppState::Scanning));

        // 2. Send Result (Done)
        let result = ScanResult {
            category,
            total_size: 1024,
            items: vec![],
            is_selected: false,
            description: "Test description".to_string(),
            root_path: PathBuf::from("/tmp"),
        };
        tx.send(ScanUpdate::Result(result)).unwrap();

        // Process update
        app.check_scan_status();

        // Verify completion
        let progress = app
            .scan_progress
            .get(&category)
            .expect("Category should exist");
        assert_eq!(progress.status, "Done");

        // Should transition to Browsing because results.len() (1) == total_categories (1)
        assert_eq!(app.results.len(), 1);
        assert!(matches!(app.state, AppState::Browsing));
        assert!(app.scan_rx.is_none());
    }
}
