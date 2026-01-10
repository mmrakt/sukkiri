use crate::model::CategoryType;
use crate::ui::app::{App, AppState};
use humansize::{BINARY, format_size};
use ratatui::{
    prelude::*,
    widgets::{BarChart, Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use std::fmt::Write as _;

pub fn render_header(f: &mut Frame, app: &App, area: Rect) {
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

pub fn render_categories_list(f: &mut Frame, app: &mut App, area: Rect) {
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

pub fn render_usage_chart(f: &mut Frame, app: &App, area: Rect) {
    let short_data: Vec<(&str, u64)> = app
        .results
        .iter()
        .map(|r| {
            let label = match r.category {
                CategoryType::XcodeJunk => "Xcode",
                CategoryType::SystemLogs => "SysLog",
                CategoryType::SystemCache => "SysCache",
                CategoryType::UserLogs => "UsrLog",
                CategoryType::UserCache => "UsrCache",
                CategoryType::BrowserCache => "Browser",
                CategoryType::Downloads => "Downlds",
                CategoryType::Trash => "Trash",
                CategoryType::DeveloperCaches => "DevCache",
                CategoryType::ScreenCapture => "Screen",
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

pub fn render_details_text(f: &mut Frame, app: &App, area: Rect) {
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

pub fn render_details(f: &mut Frame, app: &App, area: Rect) {
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_usage_chart(f, app, right_chunks[0]);
    render_details_text(f, app, right_chunks[1]);
}

pub fn render_footer(f: &mut Frame, app: &App, area: Rect) {
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

pub fn render_popup(f: &mut Frame, app: &App) {
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
