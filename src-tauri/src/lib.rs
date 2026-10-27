mod cli;
mod commands;
mod settings;
mod store;
mod timer;
mod tray;
mod usage;
mod windows;

use std::sync::atomic::Ordering;
use std::sync::Arc;

use tauri::Manager;

use crate::settings::Settings;
use crate::store::Store;
use crate::timer::{now_ms, TimerEngine};
use crate::usage::UsageService;

pub struct AppState {
    pub engine: Arc<TimerEngine>,
    pub usage: Arc<UsageService>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            cli::handle(app, &argv);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::SIZE
                        | tauri_plugin_window_state::StateFlags::POSITION
                        | tauri_plugin_window_state::StateFlags::MAXIMIZED,
                )
                .with_denylist(&["mini"])
                .build(),
        )
        .setup(|app| {
            let data_dir = app.path().data_dir()?.join("tempo");
            let store = Arc::new(
                Store::open(&data_dir).map_err(std::io::Error::other)?,
            );

            let mut settings: Settings = store.get_meta("settings").unwrap_or_default();
            settings.sanitize();
            let usage = Arc::new(UsageService::start(
                store.clone(),
                settings.usage_tracking_enabled,
                settings.idle_threshold_secs * 1000,
            ));
            let core = timer::recover(&store, settings.default_ms(), now_ms());
            let engine = Arc::new(TimerEngine::new(
                app.handle().clone(),
                store,
                settings,
                core,
            ));

            // Recovery can finish an expired timer; write that state back so a
            // second restart does not record the same session again.
            engine.persist();

            tray::setup(app, engine.clone())?;

            app.manage(AppState {
                engine: engine.clone(),
                usage,
            });

            engine.update_tray();
            engine.emit_update();
            timer::spawn_ticker(engine.clone());

            let args: Vec<String> = std::env::args().collect();
            cli::handle(app.handle(), &args);

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    let state = window.app_handle().state::<AppState>();
                    if !state.engine.quitting.load(Ordering::SeqCst) {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::start_timer,
            commands::pause_timer,
            commands::resume_timer,
            commands::toggle_timer,
            commands::reset_timer,
            commands::set_duration,
            commands::set_mode,
            commands::get_sessions,
            commands::get_todos,
            commands::add_todo,
            commands::set_todo_done,
            commands::update_todo_text,
            commands::delete_todo,
            commands::get_settings,
            commands::update_settings,
            commands::set_autostart,
            commands::get_usage_status,
            commands::get_usage_snapshot,
            commands::clear_usage_history,
            commands::open_mini,
            commands::show_main,
            commands::hide_main,
            commands::hide_mini,
            commands::quit_app,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Tempo")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                app.state::<AppState>().usage.shutdown();
            }
        });
}
