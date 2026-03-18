use crate::tui_app::{App, InputMode};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::time::Duration;

pub fn handle_events(app: &mut App) -> std::io::Result<bool> {
    if event::poll(Duration::from_millis(50))?
        && let Event::Key(key) = event::read()?
    {
        if key.kind != KeyEventKind::Press {
            return Ok(false);
        }

        match app.input_mode {
            InputMode::Normal => handle_normal_mode(app, key),
            InputMode::Search => handle_search_mode(app, key),
            InputMode::AlgorithmPopup => handle_algorithm_popup_mode(app, key),
            InputMode::MethodPopup => handle_method_popup_mode(app, key),
        }
        app.persist_state();
        return Ok(true);
    }

    Ok(false)
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Esc => {
            if app.show_detail {
                app.show_detail = false;
            } else {
                app.should_quit = true;
            }
        }
        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::PageUp => app.page_up(),
        KeyCode::PageDown => app.page_down(),
        KeyCode::Home | KeyCode::Char('g') => app.home(),
        KeyCode::End | KeyCode::Char('G') => app.end(),
        KeyCode::Enter => app.toggle_detail(),
        KeyCode::Char('/') => app.enter_search(),
        KeyCode::Char('f') => app.cycle_fit_filter(),
        KeyCode::Char('s') => app.cycle_sort_column(),
        KeyCode::Char('t') => app.cycle_theme(),
        KeyCode::Char('e') => app.cycle_electricity(),
        KeyCode::Char('R') => app.refresh_data(),
        KeyCode::Char('A') => app.open_algorithm_popup(),
        KeyCode::Char('M') => app.open_method_popup(),
        _ => {}
    }
}

fn handle_search_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Enter => app.exit_search(),
        KeyCode::Backspace => app.search_backspace(),
        KeyCode::Delete => app.search_delete(),
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => app.clear_search(),
        KeyCode::Char(c) => app.search_input(c),
        KeyCode::Up => app.move_up(),
        KeyCode::Down => app.move_down(),
        _ => {}
    }
}

fn handle_algorithm_popup_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('A') | KeyCode::Char('q') => app.close_algorithm_popup(),
        KeyCode::Up | KeyCode::Char('k') => app.algorithm_popup_up(),
        KeyCode::Down | KeyCode::Char('j') => app.algorithm_popup_down(),
        KeyCode::Char(' ') | KeyCode::Enter => app.algorithm_popup_toggle(),
        KeyCode::Char('a') => app.algorithm_popup_select_all(),
        _ => {}
    }
}

fn handle_method_popup_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('M') | KeyCode::Char('q') => app.close_method_popup(),
        KeyCode::Up | KeyCode::Char('k') => app.method_popup_up(),
        KeyCode::Down | KeyCode::Char('j') => app.method_popup_down(),
        KeyCode::Char(' ') | KeyCode::Enter => app.method_popup_toggle(),
        KeyCode::Char('a') => app.method_popup_select_all(),
        _ => {}
    }
}
