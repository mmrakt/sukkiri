use crate::model::CategoryType;
use crate::ui::app::{App, AppState};
use humansize::{BINARY, format_size};
use ratatui::{
    prelude::*,
    widgets::{
        Block, BorderType, Borders, Cell, Clear, Gauge, List, ListItem, Paragraph, Row, Table, Wrap,
    },
};

const COLOR_PRIMARY: Color = Color::Cyan;
const COLOR_SECONDARY: Color = Color::Blue;
const COLOR_ACCENT: Color = Color::Magenta;
const COLOR_BORDER: Color = Color::DarkGray;

#[allow(clippy::cast_precision_loss)]
pub fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let disk_info = app
        .disks
        .list()
        .iter()
        .find(|d| d.mount_point() == std::path::Path::new("/"));

    let (percent, label) = if let Some(disk) = disk_info {
        let total = disk.total_space();
        let available = disk.available_space();
        let used = total.saturating_sub(available);
        let ratio = if total > 0 {
            used as f64 / total as f64
        } else {
            0.0
        };
        // Clamp between 0.0 and 100.0 (Gauge expects ratio 0..1 or percent 0..100 depending on impl, usually ratio 0.0 to 1.0 for some widgets, but Ratatui Gauge takes ratio, LineGauge takes ratio. Standard Gauge uses ratio 0.0-1.0 or percent.)
        // Ratatui Gauge uses .ratio(0.0..1.0) or .percent(0..100). Let's use ratio.
        (
            ratio.clamp(0.0, 1.0),
            format!(
                "Disk: {} / {} ({:.1}% Used)",
                format_size(used, BINARY),
                format_size(total, BINARY),
                ratio * 100.0
            ),
        )
    } else {
        (0.0, "Disk: N/A".to_string())
    };

    let gauge = Gauge::default()
        .block(
            Block::default()
                .title("sukkiri v0.1.0")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(COLOR_BORDER)),
        )
        .gauge_style(Style::default().fg(COLOR_SECONDARY).bg(Color::Black))
        .ratio(percent)
        .label(label)
        .use_unicode(true);

    f.render_widget(gauge, area);
}

pub fn render_categories_list(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .results
        .iter()
        .map(|r| {
            let checkbox = if r.is_selected { "[x]" } else { "[ ]" };
            let size_str = format_size(r.total_size, BINARY);
            let content = Line::from(vec![
                Span::styled(
                    format!("{} {:<18}", checkbox, r.category.name()),
                    Style::default(),
                ),
                Span::styled(
                    format!("{size_str:>10}"),
                    Style::default().fg(COLOR_PRIMARY),
                ),
            ]);
            ListItem::new(content)
        })
        .collect();

    let total_all_size: u64 = app.results.iter().map(|r| r.total_size).sum();

    // We want to render the list, and at the bottom the total size.
    // Ratatui List doesn't have a "footer" for the block easily unless we use Block title_bottom.
    // Or we can manually render the Total line below the list if we split the area, but Block title is easier.

    let total_text = format!(" Total: {} ", format_size(total_all_size, BINARY));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(COLOR_BORDER))
                .title("Categories")
                .title_bottom(
                    Line::from(total_text).alignment(Alignment::Right).style(
                        Style::default()
                            .fg(COLOR_PRIMARY)
                            .add_modifier(Modifier::BOLD),
                    ),
                ),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(COLOR_ACCENT),
        )
        .highlight_symbol("> ");
    f.render_stateful_widget(list, area, &mut app.list_state);
}

pub fn render_details_text(f: &mut Frame, app: &App, area: Rect) {
    let selected_index = app.list_state.selected().unwrap_or(0);

    if selected_index < app.results.len() {
        let selected_result = &app.results[selected_index];

        let header_text = format!("Details: {}", selected_result.category.name());

        // Use a Table for large items
        let header_cells = ["Name", "Size", "Path"].iter().map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(COLOR_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )
        });
        let header = Row::new(header_cells).height(1).bottom_margin(1);

        let rows = selected_result.items.iter().take(20).map(|item| {
            let name = item.path.file_name().unwrap_or_default().to_string_lossy();
            // Truncate path for display
            let path_display = item.path.display().to_string();
            // Simple truncation if too long
            let path_short = if path_display.len() > 30 {
                format!(
                    "...{}",
                    &path_display[path_display.len().saturating_sub(27)..]
                )
            } else {
                path_display
            };

            let cells = vec![
                Cell::from(name),
                Cell::from(format_size(item.size, BINARY)),
                Cell::from(path_short).style(Style::default().fg(Color::DarkGray)),
            ];
            Row::new(cells).height(1)
        });

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(40),
                Constraint::Percentage(20),
                Constraint::Percentage(40),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(COLOR_BORDER))
                .title(header_text),
        )
        .column_spacing(1);

        f.render_widget(table, area);
    } else {
        f.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(COLOR_BORDER))
                .title("Details"),
            area,
        );
    }
}

pub fn render_details(f: &mut Frame, app: &App, area: Rect) {
    // Layout simplified: No chart, just details text in full area
    render_details_text(f, app, area);
}

pub fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let total_selected = app.total_selected_size();
    let footer_text = match app.state {
        AppState::Browsing => format!(
            "Total Selected: {} | [Space] Toggle [a] All [Enter] Clean [q] Quit",
            format_size(total_selected, BINARY)
        ),
        AppState::Confirming => format!(
            "CONFIRM CLEAN? Selected: {} | [y/Enter] Confirm [n/Esc] Cancel",
            format_size(total_selected, BINARY)
        ),
        AppState::Cleaning => "Cleaning... (This may take a while)".to_string(),
        AppState::Scanning => "Scanning... (Please wait)".to_string(),
        AppState::Done(_) => "Done! [Press key to continue]".to_string(),
    };

    let footer = Paragraph::new(footer_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(COLOR_BORDER)),
    );
    f.render_widget(footer, area);
}

pub fn render_popup(f: &mut Frame, app: &App) {
    if let AppState::Done(ref msg) = app.state {
        let block = Block::default()
            .title("Clean Completed")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(COLOR_BORDER));
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

#[allow(clippy::cast_precision_loss)]
pub fn render_scanning(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Overall gauge
            Constraint::Min(0),    // Detail list
        ])
        .split(area);

    // 1. Overall Gauge
    let completed_count = app.results.len() as f64;
    let total_count = app.total_categories as f64;
    let ratio = if total_count > 0.0 {
        completed_count / total_count
    } else {
        0.0
    };

    let label = format!(
        "Scanning Categories: {} / {}",
        app.results.len(),
        app.total_categories
    );

    let gauge = Gauge::default()
        .block(
            Block::default()
                .title("Scan Progress")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(COLOR_BORDER)),
        )
        .gauge_style(Style::default().fg(COLOR_PRIMARY).bg(Color::Black))
        .ratio(ratio)
        .label(label)
        .use_unicode(true);

    f.render_widget(gauge, chunks[0]);

    // 2. List of categories status
    let mut items = Vec::new();

    // Show specific order if possible, or just iterate map
    // We want to show all categories and their status

    // We can't iterate HashMap in consistent order unless we sort.
    // Let's use the hardcoded order from main/start_scan if we can, or just sort keys.
    let mut categories: Vec<CategoryType> = app.scan_progress.keys().copied().collect();
    // Sort by name for now, or arbitrary stability
    categories.sort_by_key(|c| c.name());

    for cat in categories {
        if let Some(prog) = app.scan_progress.get(&cat) {
            let spinner = if prog.status == "Done" { "✔" } else { "⠋" };
            let style = if prog.status == "Done" {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Yellow)
            };

            let content = Line::from(vec![
                Span::styled(format!("{} {:<20}", spinner, prog.category.name()), style),
                Span::raw(format!(
                    "Items: {:<5} Status: {}",
                    prog.items_count, prog.status
                )),
            ]);
            items.push(ListItem::new(content));
        }
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Details"),
    );

    f.render_widget(list, chunks[1]);
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
