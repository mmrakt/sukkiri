mod allowlist;
mod cleaner;
mod model;
mod scanner;

use allowlist::Allowlist;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use humansize::{BINARY, format_size};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use model::{CategoryType, ScanResult};
use ratatui::{
    prelude::*,
    widgets::{BarChart, Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use std::fmt::Write as _;
use std::io;
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;
use sysinfo::Disks;

enum AppState {
    Browsing,
    Confirming,
    Cleaning,
    Done(String), // Done message
}

struct App {
    results: Vec<ScanResult>,
    list_state: ListState,
    state: AppState,
    disks: Disks,
    // Channel receiver for cleaning thread results
    cleaning_rx: Option<mpsc::Receiver<Result<String, String>>>,
}

impl App {
    fn new(results: Vec<ScanResult>) -> Self {
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

    fn next(&mut self) {
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

    fn previous(&mut self) {
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

    fn toggle(&mut self) {
        if let Some(i) = self.list_state.selected()
            && i < self.results.len()
        {
            self.results[i].is_selected = !self.results[i].is_selected;
        }
    }

    fn total_selected_size(&self) -> u64 {
        self.results
            .iter()
            .filter(|r| r.is_selected)
            .map(|r| r.total_size)
            .sum()
    }

    fn clean_selected(&mut self) {
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

    fn check_cleaning_status(&mut self) {
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
        CategoryType::SystemCache,
        CategoryType::UserLogs,
        CategoryType::XcodeDerivedData,
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
            let result = scanner::scan_category(category, None, &allowlist_clone);
            if let Ok(res) = result {
                pb.finish_with_message(format!(
                    "✔ {} found {}",
                    res.category.name(),
                    format_size(res.total_size, BINARY)
                ));
                Some(res)
            } else {
                pb.finish_with_message(format!("✘ Failed to scan {}", category.name()));
                None
            }
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
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stderr>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        // Check for async cleaning results
        if let AppState::Cleaning = app.state {
            app.check_cleaning_status();
        }

        // Event polling with timeout to allow UI updates during Cleaning
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match app.state {
                AppState::Browsing => match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Down | KeyCode::Char('j') => app.next(),
                    KeyCode::Up | KeyCode::Char('k') => app.previous(),
                    KeyCode::Char(' ') => app.toggle(),
                    KeyCode::Enter => {
                        if app.total_selected_size() > 0 {
                            app.state = AppState::Confirming;
                        }
                    }
                    _ => {}
                },
                AppState::Confirming => match key.code {
                    KeyCode::Char('y') | KeyCode::Enter => app.clean_selected(),
                    KeyCode::Char('n' | 'q') | KeyCode::Esc => {
                        app.state = AppState::Browsing;
                    }
                    _ => {}
                },
                AppState::Cleaning => {
                    // Ignore text input while cleaning, but maybe allow force quit?
                    // For safety let's just wait.
                }
                AppState::Done(_) => match key.code {
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ' | 'q') => {
                        app.state = AppState::Browsing;
                    }
                    _ => {}
                },
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    render_header(f, app, chunks[0]);
    render_main_content(f, app, chunks[1]);
    render_footer(f, app, chunks[2]);
    render_popup(f, app);
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let disk_info = app
        .disks
        .list()
        .iter()
        .find(|d| d.mount_point() == std::path::Path::new("/"));

    let header_text = if let Some(disk) = disk_info {
        let total = disk.total_space();
        let available = disk.available_space();
        let used = total.saturating_sub(available);
        let percent = if total > 0 {
            #[allow(clippy::cast_precision_loss)]
            {
                (used as f64 / total as f64) * 100.0
            }
        } else {
            0.0
        };

        format!(
            "RustMacSweep v0.1.0 | Disk: {} / {} ({percent:.1}% Used)",
            format_size(used, BINARY),
            format_size(total, BINARY)
        )
    } else {
        "RustMacSweep v0.1.0 | Disk: N/A".to_string()
    };

    let title = Paragraph::new(header_text).block(Block::default().borders(Borders::ALL));
    f.render_widget(title, area);
}

fn render_main_content(f: &mut Frame, app: &mut App, area: Rect) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_categories_list(f, app, main_chunks[0]);
    render_details(f, app, main_chunks[1]);
}

fn render_categories_list(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .results
        .iter()
        .map(|r| {
            let checkbox = if r.is_selected { "[x]" } else { "[ ]" };
            let size_str = format_size(r.total_size, BINARY);
            let content = format!("{checkbox} {:<20}  {:>10}", r.category.name(), size_str);
            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Categories"))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )
        .highlight_symbol("> ");
    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_details(f: &mut Frame, app: &App, area: Rect) {
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_usage_chart(f, app, right_chunks[0]);
    render_details_text(f, app, right_chunks[1]);
}

fn render_usage_chart(f: &mut Frame, app: &App, area: Rect) {
    let short_data: Vec<(&str, u64)> = app
        .results
        .iter()
        .map(|r| {
            let label = match r.category {
                CategoryType::SystemCache => "Cache",
                CategoryType::UserLogs => "Logs",
                CategoryType::XcodeDerivedData => "Xcode",
                CategoryType::NodeModules => "Node",
                CategoryType::DockerImages => "Dockr",
            };
            let size_mb = r.total_size / 1024 / 1024;
            (label, size_mb)
        })
        .collect();

    let barchart = BarChart::default()
        .block(
            Block::default()
                .title("Storage Usage (MB)")
                .borders(Borders::ALL),
        )
        .data(&short_data)
        .bar_width(8)
        .bar_gap(2)
        .bar_style(Style::default().fg(Color::Cyan))
        .value_style(Style::default().fg(Color::White).bg(Color::Cyan));

    f.render_widget(barchart, area);
}

fn render_details_text(f: &mut Frame, app: &App, area: Rect) {
    let selected_index = app.list_state.selected().unwrap_or(0);

    if selected_index < app.results.len() {
        let selected_result = &app.results[selected_index];

        let mut details_text = format!(
            "Path: {}\n{}\n\nTop Large Items:\n",
            selected_result.root_path.display(),
            selected_result.description
        );

        for item in selected_result.items.iter().take(10) {
            let item_name = item.path.file_name().unwrap_or_default().to_string_lossy();
            let _ = writeln!(
                details_text,
                " - {item_name} ({})",
                format_size(item.size, BINARY)
            );
        }

        let details = Paragraph::new(details_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Details: {}", selected_result.category.name())),
            )
            .wrap(Wrap { trim: false });
        f.render_widget(details, area);
    } else {
        f.render_widget(
            Block::default().borders(Borders::ALL).title("Details"),
            area,
        );
    }
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let total_selected = app.total_selected_size();
    let footer_text = match app.state {
        AppState::Browsing => format!(
            "Total Selected: {} | [Space] Toggle [Enter] Clean [q] Quit",
            format_size(total_selected, BINARY)
        ),
        AppState::Confirming => format!(
            "CONFIRM CLEAN? Selected: {} | [y/Enter] Confirm [n/Esc] Cancel",
            format_size(total_selected, BINARY)
        ),
        AppState::Cleaning => "Cleaning... (This may take a while)".to_string(),
        AppState::Done(_) => "Done! [Press key to continue]".to_string(),
    };

    let footer = Paragraph::new(footer_text).block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, area);
}

fn render_popup(f: &mut Frame, app: &App) {
    if let AppState::Done(ref msg) = app.state {
        let block = Block::default()
            .title("Clean Completed")
            .borders(Borders::ALL);
        let area = centered_rect(60, 20, f.area());
        f.render_widget(Clear, area);
        f.render_widget(
            Paragraph::new(msg.clone())
                .block(block)
                .wrap(Wrap { trim: true }),
            area,
        );
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
