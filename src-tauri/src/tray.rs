use std::sync::atomic::Ordering;
use std::sync::Arc;

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{App, Manager};

use crate::timer::{TimerEngine, TrayItems};
use crate::windows;
use crate::AppState;

pub fn setup(app: &App, engine: Arc<TimerEngine>) -> tauri::Result<()> {
    let status = MenuItem::with_id(app, "status", "Ready", false, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", "Start", true, None::<&str>)?;
    let reset = MenuItem::with_id(app, "reset", "Reset", true, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "Show / hide Tempo", true, None::<&str>)?;
    let mini = MenuItem::with_id(app, "mini", "Mini window", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(
        app,
        &[&status, &toggle, &reset, &show, &mini, &separator, &quit],
    )?;

    TrayIconBuilder::with_id("main")
        .icon(
            Image::from_bytes(include_bytes!("../icons/tray-idle.png"))
                .expect("bundled tray icon is valid"),
        )
        .tooltip("Tempo")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle" => {
                if let Some(state) = app.try_state::<AppState>() {
                    state.engine.toggle();
                }
            }
            "reset" => {
                if let Some(state) = app.try_state::<AppState>() {
                    state.engine.reset();
                }
            }
            "show" => windows::toggle_main(app),
            "mini" => {
                let _ = windows::open_mini(app);
            }
            "quit" => {
                if let Some(state) = app.try_state::<AppState>() {
                    state.engine.quitting.store(true, Ordering::SeqCst);
                }
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    engine.set_tray_items(TrayItems { status, toggle });
    Ok(())
}
