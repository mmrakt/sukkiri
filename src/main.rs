mod allowlist;
mod cleaner;
mod constants;
mod model;
mod scanner;
mod ui;

use allowlist::Allowlist;
use anyhow::Result;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use humansize::{BINARY, format_size};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use model::CategoryType;
use ratatui::prelude::*;
use std::io;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use ui::app::App;

fn main() -> Result<()> {
    // === Phase 1: Parallel Scanning with Indicatif ===
    // This runs in standard CLI mode before TUI
    println!("🔍 Scanning System...");

    // Load Allowlist (cheap, so clone it for threads)
    let allowlist = Arc::new(Allowlist::load());

    let m = MultiProgress::new();
    let spinner_style = ProgressStyle::default_spinner()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
        .template("{spinner:.green} {msg}")?;

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

    let mut handles = Vec::new();

    for category in categories {
        let pb = m.add(ProgressBar::new_spinner());
        pb.set_style(spinner_style.clone());
        pb.set_message(format!("Scanning {}...", category.name()));
        pb.enable_steady_tick(Duration::from_millis(100));

        let allowlist_clone = Arc::clone(&allowlist);

        // Spawn thread for this category
        let handle = thread::spawn(move || {
            let res = scanner::scan_category(category, None, &allowlist_clone);
            pb.finish_with_message(format!(
                "✔ {} found {}",
                res.category.name(),
                format_size(res.total_size, BINARY)
            ));
            Some(res)
        });

        handles.push(handle);
    }

    // Join all threads
    let mut results = Vec::new();
    for handle in handles {
        if let Ok(Some(res)) = handle.join() {
            results.push(res);
        }
    }

    // Small pause to let user see results
    println!("\n✨ Scan complete. Launching dashboard...");
    thread::sleep(Duration::from_secs(1));

    // === Phase 2: TUI Dashboard ===
    enable_raw_mode()?;
    let mut stderr = io::stderr();
    execute!(stderr, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stderr);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(results);
    let res = ui::run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}
