//! TUI rendering

use crate::app::{App, AppMode, SearchMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(frame.area());

    render_search_bar(frame, app, chunks[0]);
    render_main(frame, app, chunks[1]);
    render_status(frame, app, chunks[2]);
}

fn render_search_bar(frame: &mut Frame, app: &App, area: Rect) {
    let mode_indicator = match app.search_mode {
        SearchMode::Bm25 => "[BM25]",
        SearchMode::Vector => "[VEC]",
        SearchMode::Hybrid => "[HYB]",
    };

    let input = Paragraph::new(format!("{} {}", mode_indicator, app.query))
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Search (Tab to change mode) "),
        );

    frame.render_widget(input, area);

    if app.mode == AppMode::Search {
        frame.set_cursor_position((
            area.x + mode_indicator.len() as u16 + app.cursor_pos as u16 + 2,
            area.y + 1,
        ));
    }
}

fn render_main(frame: &mut Frame, app: &App, area: Rect) {
    match app.mode {
        AppMode::Search | AppMode::Results => {
            render_results(frame, app, area);
        }
        AppMode::Preview => {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(area);

            render_results(frame, app, chunks[0]);
            render_preview(frame, app, chunks[1]);
        }
        AppMode::Collections => {
            render_collections(frame, app, area);
        }
        AppMode::Help => {
            render_help(frame, area);
        }
    }
}

fn render_results(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .results
        .iter()
        .enumerate()
        .skip(app.scroll_offset)
        .take(area.height as usize - 2)
        .map(|(i, result)| {
            let style = if i == app.selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let score_color = if result.score >= 0.7 {
                Color::Green
            } else if result.score >= 0.4 {
                Color::Yellow
            } else {
                Color::DarkGray
            };

            let line = Line::from(vec![
                Span::styled(
                    format!("{:>3}% ", (result.score * 100.0) as u32),
                    Style::default().fg(score_color),
                ),
                Span::styled(&result.display_path, Style::default().fg(Color::Cyan)),
                Span::raw(" - "),
                Span::raw(&result.title),
            ]);

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Results ({}) ", app.results.len())),
    );

    frame.render_widget(list, area);
}

fn render_preview(frame: &mut Frame, app: &App, area: Rect) {
    let content = app
        .preview_content
        .as_deref()
        .unwrap_or("No preview available");

    let lines: Vec<Line> = content
        .lines()
        .skip(app.preview_scroll)
        .take(area.height as usize - 2)
        .enumerate()
        .map(|(i, line)| {
            Line::from(vec![
                Span::styled(
                    format!("{:>4} ", app.preview_scroll + i + 1),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(line),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" Preview "))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

fn render_collections(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .collections
        .iter()
        .enumerate()
        .map(|(i, coll)| {
            let style = if i == app.collections_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let is_filtered = app.collection_filter.as_ref() == Some(coll);
            let marker = if is_filtered { "[*] " } else { "[ ] " };

            let line = Line::from(vec![
                Span::styled(marker, Style::default().fg(Color::Yellow)),
                Span::raw(coll),
            ]);

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Collections (Enter to filter, Esc to close) "),
    );

    frame.render_widget(list, area);
}

fn render_help(frame: &mut Frame, area: Rect) {
    let help_text = vec![
        "Agentroot TUI - Keyboard Shortcuts",
        "",
        "Search Mode:",
        "  Type to search",
        "  Tab       - Cycle search mode (BM25/Vector/Hybrid)",
        "  Enter     - View results",
        "  c         - Collections",
        "  ?         - This help screen",
        "  Esc       - Clear query / Quit",
        "",
        "Results Mode:",
        "  j/k       - Navigate up/down",
        "  Enter     - Preview document",
        "  y         - Copy file path to clipboard",
        "  c         - Toggle collection filter",
        "  /         - Return to search",
        "  Esc/q     - Back to search",
        "",
        "Preview Mode:",
        "  j/k       - Scroll up/down",
        "  PgUp/PgDn - Page up/down",
        "  Esc/q     - Back to results",
        "",
        "Collections Mode:",
        "  j/k       - Navigate",
        "  Enter     - Toggle filter",
        "  Esc       - Close",
    ];

    let lines: Vec<Line> = help_text
        .iter()
        .map(|&text| Line::from(Span::raw(text)))
        .collect();

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Help (q/Esc to close) "),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

fn render_status(frame: &mut Frame, app: &App, area: Rect) {
    let status = if app.is_loading {
        "Loading...".to_string()
    } else if let Some(ref msg) = app.status_message {
        msg.clone()
    } else {
        let mode_help = match app.mode {
            AppMode::Search => {
                "Enter: results | Tab: mode | c: collections | ?: help | Esc: clear/quit"
            }
            AppMode::Results => "j/k: navigate | Enter: preview | y: copy | c: filter | /: search",
            AppMode::Preview => "j/k: scroll | q: back",
            AppMode::Collections => "j/k: navigate | Enter: select | Esc: back",
            AppMode::Help => "q/Esc: back",
        };
        mode_help.to_string()
    };

    let paragraph = Paragraph::new(status).style(Style::default().fg(Color::DarkGray));

    frame.render_widget(paragraph, area);
}
