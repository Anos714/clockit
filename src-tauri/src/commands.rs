use std::sync::atomic::Ordering;

use tauri::{AppHandle, Manager, State};

use crate::settings::{Settings, SettingsView};
use crate::store::{SessionRecord, TodoRecord};
use crate::timer::{now_ms, TimerMode, TimerSnapshot};
use crate::windows;
use crate::AppState;

const MAX_TODO_CHARS: usize = 280;

fn clean_todo_text(text: String) -> Result<String, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("Task cannot be empty".to_string());
    }
    if text.chars().count() > MAX_TODO_CHARS {
        return Err(format!(
            "Task must be {MAX_TODO_CHARS} characters or fewer"
        ));
    }
    Ok(text.to_string())
}

#[tauri::command]
pub fn get_snapshot(state: State<'_, AppState>) -> TimerSnapshot {
    state.engine.snapshot()
}

#[tauri::command]
pub fn start_timer(state: State<'_, AppState>, seconds: Option<u64>) -> TimerSnapshot {
    state.engine.start(seconds);
    state.engine.snapshot()
}

#[tauri::command]
pub fn pause_timer(state: State<'_, AppState>) -> TimerSnapshot {
    state.engine.pause();
    state.engine.snapshot()
}

#[tauri::command]
pub fn resume_timer(state: State<'_, AppState>) -> TimerSnapshot {
    state.engine.resume();
    state.engine.snapshot()
}

#[tauri::command]
pub fn toggle_timer(state: State<'_, AppState>) -> TimerSnapshot {
    state.engine.toggle();
    state.engine.snapshot()
}

#[tauri::command]
pub fn reset_timer(state: State<'_, AppState>) -> TimerSnapshot {
    state.engine.reset();
    state.engine.snapshot()
}

#[tauri::command]
pub fn set_duration(state: State<'_, AppState>, seconds: u64) -> TimerSnapshot {
    state.engine.set_duration(seconds);
    state.engine.snapshot()
}

#[tauri::command]
pub fn set_mode(state: State<'_, AppState>, mode: TimerMode) -> TimerSnapshot {
    state.engine.set_mode(mode);
    state.engine.snapshot()
}

#[tauri::command]
pub fn get_sessions(
    state: State<'_, AppState>,
    limit: Option<u32>,
) -> Result<Vec<SessionRecord>, String> {
    state.engine.sessions(limit.unwrap_or(20).clamp(1, 200))
}

#[tauri::command]
pub fn get_todos(state: State<'_, AppState>) -> Result<Vec<TodoRecord>, String> {
    state.engine.store.list_todos()
}

#[tauri::command]
pub fn add_todo(state: State<'_, AppState>, text: String) -> Result<TodoRecord, String> {
    let text = clean_todo_text(text)?;
    state.engine.store.add_todo(&text, now_ms())
}

#[tauri::command]
pub fn set_todo_done(
    state: State<'_, AppState>,
    id: i64,
    done: bool,
) -> Result<TodoRecord, String> {
    state.engine.store.set_todo_done(id, done, now_ms())
}

#[tauri::command]
pub fn update_todo_text(
    state: State<'_, AppState>,
    id: i64,
    text: String,
) -> Result<TodoRecord, String> {
    let text = clean_todo_text(text)?;
    state.engine.store.update_todo_text(id, &text, now_ms())
}

#[tauri::command]
pub fn delete_todo(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state.engine.store.delete_todo(id)
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> SettingsView {
    state.engine.settings_view()
}

#[tauri::command]
pub fn update_settings(
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<SettingsView, String> {
    state.engine.update_settings(settings)
}

#[tauri::command]
pub fn set_autostart(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<SettingsView, String> {
    state.engine.set_autostart(enabled)
}

#[tauri::command]
pub fn open_mini(app: AppHandle) -> Result<(), String> {
    windows::open_mini(&app)
}

#[tauri::command]
pub fn show_main(app: AppHandle) {
    windows::show_main(&app);
}

#[tauri::command]
pub fn hide_main(app: AppHandle) {
    windows::hide_main(&app);
}

#[tauri::command]
pub fn hide_mini(app: AppHandle) {
    if let Some(window) = app.get_webview_window("mini") {
        let _ = window.hide();
    }
}

#[tauri::command]
pub fn quit_app(app: AppHandle, state: State<'_, AppState>) {
    state.engine.quitting.store(true, Ordering::SeqCst);
    app.exit(0);
}

#[cfg(test)]
mod tests {
    use super::clean_todo_text;

    #[test]
    fn todo_text_is_trimmed_and_validated() {
        assert_eq!(clean_todo_text("  ship it  ".into()).unwrap(), "ship it");
        assert!(clean_todo_text("   ".into()).is_err());
        assert!(clean_todo_text("x".repeat(281)).is_err());
        assert!(clean_todo_text("x".repeat(280)).is_ok());
    }
}
