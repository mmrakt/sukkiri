mod allowlist;
mod cleaner;
mod constants;
mod model;
mod scanner;
mod ui;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use std::io;
use ui::app::App;

#[derive(Parser)]
#[command(version, about, long_about = None, disable_version_flag = true)]
struct Cli {
    /// Print version information
    #[arg(short = 'v', long = "version", action = clap::ArgAction::Version)]
    version: Option<bool>,
}

fn main() -> Result<()> {
    let _cli = Cli::parse();

    enable_raw_mode()?;
    let mut stderr = io::stderr();
    execute!(stderr, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stderr);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new_scanning();
    app.start_scan();

    let res = ui::run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}
