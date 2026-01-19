//! TUI event handling

use crate::app::{App, AppMode};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use std::time::Duration;

pub async fn handle_events(app: &mut App) -> Result<()> {
    if event::poll(Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            match app.mode {
                AppMode::Search => handle_search_input(app, key),
                AppMode::Results => handle_results_input(app, key),
                AppMode::Preview => handle_preview_input(app, key),
            }
        }
    }
    Ok(())
}

fn handle_search_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            if !app.query.is_empty() {
                app.query.clear();
                app.cursor_pos = 0;
                app.results.clear();
            } else {
                app.should_quit = true;
            }
        }
        KeyCode::Enter => {
            if !app.results.is_empty() {
                app.mode = AppMode::Results;
            }
        }
        KeyCode::Down => {
            app.mode = AppMode::Results;
        }
        KeyCode::Tab => {
            app.cycle_search_mode();
            app.search();
        }
        KeyCode::Char(c) => {
            app.query.insert(app.cursor_pos, c);
            app.cursor_pos += 1;
            app.search();
        }
        KeyCode::Backspace => {
            if app.cursor_pos > 0 {
                app.cursor_pos -= 1;
                app.query.remove(app.cursor_pos);
                app.search();
            }
        }
        KeyCode::Left => {
            if app.cursor_pos > 0 {
                app.cursor_pos -= 1;
            }
        }
        KeyCode::Right => {
            if app.cursor_pos < app.query.len() {
                app.cursor_pos += 1;
            }
        }
        _ => {}
    }
}

fn handle_results_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = AppMode::Search;
        }
        KeyCode::Char('/') => {
            app.mode = AppMode::Search;
        }
        KeyCode::Enter => {
            app.load_preview();
            app.mode = AppMode::Preview;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.select_next();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.select_prev();
        }
        KeyCode::Char('y') => {
            if let Some(result) = app.results.get(app.selected) {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(&result.filepath);
                    app.status_message = Some("Copied path to clipboard".to_string());
                }
            }
        }
        _ => {}
    }
}

fn handle_preview_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = AppMode::Results;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.preview_scroll += 1;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.preview_scroll = app.preview_scroll.saturating_sub(1);
        }
        KeyCode::PageDown => {
            app.preview_scroll += 20;
        }
        KeyCode::PageUp => {
            app.preview_scroll = app.preview_scroll.saturating_sub(20);
        }
        _ => {}
    }
}
