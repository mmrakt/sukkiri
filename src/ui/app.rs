use crate::cleaner;
use crate::model::ScanResult;
use anyhow::Result;
use humansize::{BINARY, format_size};
use ratatui::widgets::ListState;
use std::sync::mpsc;
use std::thread;
use sysinfo::Disks;

pub enum AppState {
    Browsing,
    Confirming,
    Cleaning,
    Done(String), // Done message
}

pub struct App {
    pub results: Vec<ScanResult>,
    pub list_state: ListState,
    pub state: AppState,
    pub disks: Disks,
    // Channel receiver for cleaning thread results
    pub cleaning_rx: Option<mpsc::Receiver<Result<String, String>>>,
}

impl App {
    pub fn new(results: Vec<ScanResult>) -> Self {
        let mut state = ListState::default();
        if !results.is_empty() {
            state.select(Some(0));
        }

        let disks = Disks::new_with_refreshed_list();

        Self {
            results,
            list_state: state,
            state: AppState::Browsing,
            disks,
            cleaning_rx: None,
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
}
