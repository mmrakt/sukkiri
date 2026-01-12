pub mod app;
pub mod components;

use crate::ui::app::{App, AppState};
use crate::ui::components::{
    render_categories_list, render_details, render_footer, render_header, render_popup,
    render_scanning,
};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::prelude::*;
use std::time::Duration;

pub fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    render_header(f, app, chunks[0]);

    if let AppState::Scanning = app.state {
        render_scanning(f, app, chunks[1]);
    } else {
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(chunks[1]);

        render_categories_list(f, app, main_chunks[0]);
        render_details(f, app, main_chunks[1]);
    }

    render_footer(f, app, chunks[2]);
    render_popup(f, app);
}

pub fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stderr>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        // Check for async cleaning results
        if let AppState::Cleaning = app.state {
            app.check_cleaning_status();
        }

        // Check for scanning results
        if let AppState::Scanning = app.state {
            app.check_scan_status();
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
                AppState::Scanning => {
                    if let KeyCode::Char('q') | KeyCode::Esc = key.code {
                        // Allow early exit?
                        return Ok(());
                    }
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
