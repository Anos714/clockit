use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::image::Image;
use tauri::menu::MenuItem;
use tauri::{AppHandle, Emitter, Wry};
use tauri_plugin_autostart::ManagerExt as AutostartExt;
use tauri_plugin_notification::NotificationExt;

use crate::settings::{Settings, SettingsView};
use crate::store::{NewSession, SessionRecord, Store};

pub const CHIME: &[u8] = include_bytes!("../assets/chime.wav");
const TRAY_IDLE: &[u8] = include_bytes!("../icons/tray-idle.png");
const TRAY_ACTIVE: &[u8] = include_bytes!("../icons/tray-active.png");

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimerStatus {
    Idle,
    Running,
    Paused,
    Finished,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimerMode {
    #[default]
    Countdown,
    Stopwatch,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimerCore {
    pub status: TimerStatus,
    #[serde(default)]
    pub mode: TimerMode,
    pub planned_ms: u64,
    pub remaining_ms: u64,
    pub ends_at_ms: u64,
    pub session_started_at_ms: Option<u64>,
    #[serde(default)]
    pub stopwatch_base_ms: u64,
    #[serde(default)]
    pub stopwatch_started_at_ms: u64,
}

impl TimerCore {
    pub fn new(planned_ms: u64) -> Self {
        let planned_ms = planned_ms.max(1000);
        Self {
            status: TimerStatus::Idle,
            mode: TimerMode::Countdown,
            planned_ms,
            remaining_ms: planned_ms,
            ends_at_ms: 0,
            session_started_at_ms: None,
            stopwatch_base_ms: 0,
            stopwatch_started_at_ms: 0,
        }
    }

    pub fn start(&mut self, now: u64, duration_ms: u64) {
        let duration_ms = duration_ms.max(1000);
        self.status = TimerStatus::Running;
        self.planned_ms = duration_ms;
        self.remaining_ms = duration_ms;
        self.ends_at_ms = now + duration_ms;
        self.session_started_at_ms = Some(now);
    }

    pub fn start_stopwatch(&mut self, now: u64) {
        match self.status {
            TimerStatus::Running => return,
            TimerStatus::Paused => {
                self.stopwatch_started_at_ms = now;
                self.status = TimerStatus::Running;
            }
            TimerStatus::Idle | TimerStatus::Finished => {
                self.stopwatch_base_ms = 0;
                self.stopwatch_started_at_ms = now;
                self.ends_at_ms = 0;
                self.remaining_ms = 0;
                self.session_started_at_ms = Some(now);
                self.status = TimerStatus::Running;
            }
        }
    }

    pub fn pause(&mut self, now: u64) {
        if self.status != TimerStatus::Running {
            return;
        }
        let remaining = self.ends_at_ms.saturating_sub(now);
        self.remaining_ms = remaining;
        self.ends_at_ms = 0;
        if remaining == 0 {
            self.status = TimerStatus::Finished;
        } else {
            self.status = TimerStatus::Paused;
        }
    }

    pub fn pause_stopwatch(&mut self, now: u64) {
        if self.status != TimerStatus::Running {
            return;
        }
        self.stopwatch_base_ms = self.stopwatch_elapsed_at(now);
        self.stopwatch_started_at_ms = 0;
        self.status = TimerStatus::Paused;
    }

    pub fn resume(&mut self, now: u64) {
        if self.status == TimerStatus::Paused && self.remaining_ms > 0 {
            self.ends_at_ms = now + self.remaining_ms;
            self.status = TimerStatus::Running;
        }
    }

    pub fn resume_stopwatch(&mut self, now: u64) {
        if self.status == TimerStatus::Paused {
            self.stopwatch_started_at_ms = now;
            self.status = TimerStatus::Running;
        }
    }

    pub fn tick(&mut self, now: u64) -> bool {
        if self.mode == TimerMode::Countdown
            && self.status == TimerStatus::Running
            && now >= self.ends_at_ms
        {
            self.status = TimerStatus::Finished;
            self.remaining_ms = 0;
            return true;
        }
        false
    }

    pub fn reset(&mut self) {
        self.status = TimerStatus::Idle;
        self.remaining_ms = self.planned_ms;
        self.ends_at_ms = 0;
        self.session_started_at_ms = None;
        self.stopwatch_base_ms = 0;
        self.stopwatch_started_at_ms = 0;
    }

    pub fn set_mode(&mut self, mode: TimerMode) {
        if self.mode == mode {
            return;
        }
        self.mode = mode;
        self.reset();
    }

    pub fn set_duration(&mut self, duration_ms: u64) {
        let duration_ms = duration_ms.max(1000);
        self.planned_ms = duration_ms;
        self.remaining_ms = duration_ms;
    }

    pub fn stopwatch_elapsed_at(&self, now: u64) -> u64 {
        if self.status == TimerStatus::Running && self.stopwatch_started_at_ms > 0 {
            self.stopwatch_base_ms
                .saturating_add(now.saturating_sub(self.stopwatch_started_at_ms))
        } else {
            self.stopwatch_base_ms
        }
    }

    pub fn remaining_at(&self, now: u64) -> u64 {
        match self.mode {
            TimerMode::Countdown => {
                if self.status == TimerStatus::Running {
                    self.ends_at_ms.saturating_sub(now)
                } else {
                    self.remaining_ms
                }
            }
            TimerMode::Stopwatch => 0,
        }
    }

    pub fn elapsed_at(&self, now: u64) -> u64 {
        match self.mode {
            TimerMode::Countdown => self.planned_ms.saturating_sub(self.remaining_at(now)),
            TimerMode::Stopwatch => self.stopwatch_elapsed_at(now),
        }
    }

    pub fn snapshot(&self, now: u64) -> TimerSnapshot {
        let remaining_ms = self.remaining_at(now);
        let elapsed_ms = self.elapsed_at(now);
        let progress = if self.mode == TimerMode::Stopwatch || self.planned_ms == 0 {
            0.0
        } else {
            (elapsed_ms as f64 / self.planned_ms as f64).clamp(0.0, 1.0)
        };
        TimerSnapshot {
            status: self.status,
            mode: self.mode,
            planned_ms: self.planned_ms,
            remaining_ms,
            elapsed_ms,
            progress,
            ends_at_ms: (self.mode == TimerMode::Countdown
                && self.status == TimerStatus::Running)
                .then_some(self.ends_at_ms),
            session_started_at_ms: self.session_started_at_ms,
            stopwatch_base_ms: self.stopwatch_base_ms,
            stopwatch_started_at_ms: (self.mode == TimerMode::Stopwatch
                && self.status == TimerStatus::Running
                && self.stopwatch_started_at_ms > 0)
                .then_some(self.stopwatch_started_at_ms),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerSnapshot {
    pub status: TimerStatus,
    pub mode: TimerMode,
    pub planned_ms: u64,
    pub remaining_ms: u64,
    pub elapsed_ms: u64,
    pub progress: f64,
    pub ends_at_ms: Option<u64>,
    pub session_started_at_ms: Option<u64>,
    pub stopwatch_base_ms: u64,
    pub stopwatch_started_at_ms: Option<u64>,
}

#[derive(Clone)]
pub struct TrayItems {
    pub status: MenuItem<Wry>,
    pub toggle: MenuItem<Wry>,
}

pub struct TimerEngine {
    pub core: Mutex<TimerCore>,
    pub settings: Mutex<Settings>,
    pub store: Arc<Store>,
    pub app: AppHandle,
    pub quitting: AtomicBool,
    tray_items: Mutex<Option<TrayItems>>,
    last_label: Mutex<(String, String, String)>,
    last_icon_active: Mutex<Option<bool>>,
}

impl TimerEngine {
    pub fn new(app: AppHandle, store: Arc<Store>, settings: Settings, core: TimerCore) -> Self {
        Self {
            core: Mutex::new(core),
            settings: Mutex::new(settings),
            store,
            app,
            quitting: AtomicBool::new(false),
            tray_items: Mutex::new(None),
            last_label: Mutex::new((String::new(), String::new(), String::new())),
            last_icon_active: Mutex::new(None),
        }
    }

    pub fn set_tray_items(&self, items: TrayItems) {
        *self.tray_items.lock() = Some(items);
        self.update_tray();
    }

    pub fn snapshot(&self) -> TimerSnapshot {
        let core = self.core.lock();
        core.snapshot(now_ms())
    }

    pub fn start(&self, seconds: Option<u64>) {
        let now = now_ms();
        let fallback_ms = self.settings.lock().default_ms();
        let mut core = self.core.lock();
        match core.mode {
            TimerMode::Stopwatch => {
                if core.status == TimerStatus::Running {
                    return;
                }
                core.start_stopwatch(now);
            }
            TimerMode::Countdown => match core.status {
                TimerStatus::Running => return,
                TimerStatus::Paused if seconds.is_none() => core.resume(now),
                _ => {
                    let duration = seconds
                        .map(|s| s.clamp(1, 86_400) * 1000)
                        .unwrap_or(if core.planned_ms >= 1000 {
                            core.planned_ms
                        } else {
                            fallback_ms
                        });
                    core.start(now, duration);
                }
            },
        }
        drop(core);
        self.after_change();
    }

    pub fn pause(&self) {
        let now = now_ms();
        let completed = {
            let mut core = self.core.lock();
            if core.status != TimerStatus::Running {
                return;
            }
            match core.mode {
                TimerMode::Stopwatch => {
                    core.pause_stopwatch(now);
                    None
                }
                TimerMode::Countdown => {
                    let planned = core.planned_ms;
                    let started = core.session_started_at_ms;
                    let ended = core.ends_at_ms;
                    core.pause(now);
                    (core.status == TimerStatus::Finished)
                        .then_some((planned, started, ended))
                }
            }
        };
        match completed {
            Some((planned, started, ended)) => {
                self.record_completed(planned, started, ended);
                self.on_completed();
            }
            None => self.after_change(),
        }
    }

    pub fn resume(&self) {
        let now = now_ms();
        let changed = {
            let mut core = self.core.lock();
            if core.status != TimerStatus::Paused {
                return;
            }
            match core.mode {
                TimerMode::Stopwatch => {
                    core.resume_stopwatch(now);
                    true
                }
                TimerMode::Countdown => {
                    if core.remaining_ms == 0 {
                        false
                    } else {
                        core.resume(now);
                        true
                    }
                }
            }
        };
        if changed {
            self.after_change();
        }
    }

    pub fn toggle(&self) {
        let status = self.core.lock().status;
        match status {
            TimerStatus::Running => self.pause(),
            TimerStatus::Paused => self.resume(),
            TimerStatus::Idle | TimerStatus::Finished => self.start(None),
        }
    }

    pub fn reset(&self) {
        let now = now_ms();
        let session = {
            let mut core = self.core.lock();
            let session = self.partial_session(&core, now);
            core.reset();
            session
        };
        if let Some(session) = session {
            let _ = self.store.insert_session(&session);
        }
        self.after_change();
    }

    pub fn set_duration(&self, seconds: u64) {
        let now = now_ms();
        let seconds = seconds.clamp(1, 86_400);
        let session = {
            let mut core = self.core.lock();
            let session = self.partial_session(&core, now);
            core.reset();
            core.set_duration(seconds * 1000);
            session
        };
        if let Some(session) = session {
            let _ = self.store.insert_session(&session);
        }
        self.after_change();
    }

    pub fn set_mode(&self, mode: TimerMode) {
        let now = now_ms();
        let (changed, session) = {
            let mut core = self.core.lock();
            if core.mode == mode {
                (false, None)
            } else {
                let session = self.partial_session(&core, now);
                core.set_mode(mode);
                (true, session)
            }
        };
        if !changed {
            return;
        }
        if let Some(session) = session {
            let _ = self.store.insert_session(&session);
        }
        self.after_change();
    }

    fn partial_session(&self, core: &TimerCore, now: u64) -> Option<NewSession> {
        let active = matches!(core.status, TimerStatus::Running | TimerStatus::Paused);
        let elapsed = core.elapsed_at(now);
        if !active || elapsed < 1000 {
            return None;
        }
        let started = core.session_started_at_ms.unwrap_or(now.saturating_sub(elapsed));
        let (planned_seconds, status) = match core.mode {
            TimerMode::Countdown => (core.planned_ms / 1000, "cancelled"),
            TimerMode::Stopwatch => (elapsed / 1000, "stopwatch"),
        };
        Some(NewSession {
            planned_seconds,
            actual_seconds: elapsed / 1000,
            started_at_ms: started as i64,
            ended_at_ms: now as i64,
            status,
        })
    }

    pub fn finish_if_due(&self, now: u64) -> bool {
        let (planned, started, ended) = {
            let mut core = self.core.lock();
            if core.mode != TimerMode::Countdown || core.status != TimerStatus::Running {
                return false;
            }
            let planned = core.planned_ms;
            let started = core.session_started_at_ms;
            let ended = core.ends_at_ms;
            if !core.tick(now) {
                return false;
            }
            core.session_started_at_ms = None;
            (planned, started, ended)
        };
        self.record_completed(planned, started, ended);
        self.on_completed();
        true
    }

    fn record_completed(&self, planned_ms: u64, started: Option<u64>, ended_ms: u64) {
        let started = started.unwrap_or(ended_ms.saturating_sub(planned_ms));
        let session = NewSession {
            planned_seconds: planned_ms / 1000,
            actual_seconds: planned_ms / 1000,
            started_at_ms: started as i64,
            ended_at_ms: ended_ms as i64,
            status: "completed",
        };
        let _ = self.store.insert_session(&session);
    }

    fn on_completed(&self) {
        let snapshot = self.snapshot();
        let planned_minutes = snapshot.planned_ms / 60_000;
        let body = if planned_minutes >= 1 {
            format!("{planned_minutes}-minute focus session complete.")
        } else {
            "Your timer finished.".to_string()
        };

        {
            let settings = self.settings.lock();
            if settings.notifications_enabled {
                if let Err(error) = self
                    .app
                    .notification()
                    .builder()
                    .title("Tempo - timer complete")
                    .body(&body)
                    .show()
                {
                    eprintln!("notification failed: {error}");
                }
            }
            if settings.sound_enabled {
                play_chime();
            }
        }

        let _ = self.app.emit("timer://finished", &snapshot);
        self.after_change();
    }

    pub fn settings_view(&self) -> SettingsView {
        SettingsView {
            settings: self.settings.lock().clone(),
            autostart_enabled: self.app.autolaunch().is_enabled().unwrap_or(false),
        }
    }

    pub fn update_settings(&self, mut settings: Settings) -> Result<SettingsView, String> {
        settings.sanitize();
        self.store.set_meta("settings", &settings)?;

        *self.settings.lock() = settings.clone();
        {
            let mut core = self.core.lock();
            if core.status == TimerStatus::Idle && core.mode == TimerMode::Countdown {
                core.set_duration(settings.default_ms());
            }
        }

        self.after_change();
        let view = self.settings_view();
        let _ = self.app.emit("settings://changed", &view);
        Ok(view)
    }

    pub fn set_autostart(&self, enabled: bool) -> Result<SettingsView, String> {
        let autolaunch = self.app.autolaunch();
        let result = if enabled {
            autolaunch.enable()
        } else {
            autolaunch.disable()
        };
        result.map_err(|e| format!("could not update autostart: {e}"))?;
        let view = self.settings_view();
        let _ = self.app.emit("settings://changed", &view);
        Ok(view)
    }

    pub fn sessions(&self, limit: u32) -> Result<Vec<SessionRecord>, String> {
        self.store.list_sessions(limit)
    }

    pub fn persist(&self) {
        let core = self.core.lock().clone();
        if let Err(error) = self.store.save_timer(&core) {
            eprintln!("could not persist timer: {error}");
        }
    }

    pub fn emit_update(&self) {
        let _ = self.app.emit("timer://update", self.snapshot());
    }

    fn after_change(&self) {
        self.persist();
        self.emit_update();
        self.update_tray();
    }

    pub fn update_tray(&self) {
        let snapshot = self.snapshot();
        let (status_text, toggle_text) = tray_labels(&snapshot);
        let tooltip = match snapshot.mode {
            TimerMode::Stopwatch => match snapshot.status {
                TimerStatus::Running => {
                    format!("Tempo - stopwatch {}", format_elapsed(snapshot.elapsed_ms))
                }
                TimerStatus::Paused => format!(
                    "Tempo - stopwatch paused at {}",
                    format_elapsed(snapshot.elapsed_ms)
                ),
                _ => "Tempo - stopwatch ready".to_string(),
            },
            TimerMode::Countdown => match snapshot.status {
                TimerStatus::Running => {
                    format!("Tempo - {} left", format_clock(snapshot.remaining_ms))
                }
                TimerStatus::Paused => {
                    format!("Tempo - paused at {}", format_clock(snapshot.remaining_ms))
                }
                TimerStatus::Finished => "Tempo - finished".to_string(),
                TimerStatus::Idle => "Tempo - ready".to_string(),
            },
        };

        let changed = {
            let mut last = self.last_label.lock();
            if last.0 == status_text && last.1 == toggle_text && last.2 == tooltip {
                false
            } else {
                *last = (status_text.clone(), toggle_text.clone(), tooltip.clone());
                true
            }
        };

        if changed {
            if let Some(items) = self.tray_items.lock().clone() {
                let app = self.app.clone();
                let _ = self.app.run_on_main_thread(move || {
                    let _ = items.status.set_text(status_text);
                    let _ = items.toggle.set_text(toggle_text);
                    if let Some(tray) = app.tray_by_id("main") {
                        let _ = tray.set_tooltip(Some(tooltip));
                    }
                });
            }
        }

        let active = snapshot.status == TimerStatus::Running;
        let icon_changed = {
            let mut last = self.last_icon_active.lock();
            if *last == Some(active) {
                false
            } else {
                *last = Some(active);
                true
            }
        };
        if icon_changed {
            let bytes: &'static [u8] = if active { TRAY_ACTIVE } else { TRAY_IDLE };
            let app = self.app.clone();
            let _ = self.app.run_on_main_thread(move || {
                if let Some(tray) = app.tray_by_id("main") {
                    if let Ok(image) = Image::from_bytes(bytes) {
                        let _ = tray.set_icon(Some(image));
                    }
                }
            });
        }
    }
}

fn tray_labels(snapshot: &TimerSnapshot) -> (String, String) {
    match snapshot.mode {
        TimerMode::Stopwatch => match snapshot.status {
            TimerStatus::Running => (
                format!("{} - stopwatch", format_elapsed(snapshot.elapsed_ms)),
                "Pause".to_string(),
            ),
            TimerStatus::Paused => (
                format!("{} - paused", format_elapsed(snapshot.elapsed_ms)),
                "Resume".to_string(),
            ),
            _ => ("Stopwatch ready".to_string(), "Start".to_string()),
        },
        TimerMode::Countdown => match snapshot.status {
            TimerStatus::Running => (
                format!("{} left - running", format_clock(snapshot.remaining_ms)),
                "Pause".to_string(),
            ),
            TimerStatus::Paused => (
                format!("{} left - paused", format_clock(snapshot.remaining_ms)),
                "Resume".to_string(),
            ),
            TimerStatus::Finished => ("Finished".to_string(), "Start again".to_string()),
            TimerStatus::Idle => (
                format!("Ready - {}", format_clock(snapshot.planned_ms)),
                "Start".to_string(),
            ),
        },
    }
}

pub fn format_clock(ms: u64) -> String {
    let total = ms.div_ceil(1000);
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

pub fn format_elapsed(ms: u64) -> String {
    let total = ms / 1000;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

fn play_chime() {
    let path = std::env::temp_dir().join("tempo-chime.wav");
    if !path.exists() {
        if let Err(error) = std::fs::write(&path, CHIME) {
            eprintln!("could not write chime: {error}");
            return;
        }
    }
    for player in ["pw-play", "paplay"] {
        match Command::new(player)
            .arg(&path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(_) => return,
            Err(_) => continue,
        }
    }
    eprintln!("no audio player found (looked for pw-play and paplay)");
}

pub fn recover(store: &Store, default_ms: u64, now: u64) -> TimerCore {
    let mut core = match store.load_timer() {
        Some(core) => core,
        None => return TimerCore::new(default_ms),
    };

    if core.planned_ms < 1000 {
        core.planned_ms = default_ms.max(1000);
        if core.mode == TimerMode::Countdown && core.status == TimerStatus::Idle {
            core.remaining_ms = core.planned_ms;
        }
    }

    match core.mode {
        TimerMode::Stopwatch => {
            core.ends_at_ms = 0;
            core.remaining_ms = 0;
            if core.status == TimerStatus::Running && core.stopwatch_started_at_ms == 0 {
                core.status = TimerStatus::Paused;
            }
            if core.status != TimerStatus::Running {
                core.stopwatch_started_at_ms = 0;
            }
        }
        TimerMode::Countdown => {
            if core.status == TimerStatus::Running && core.ends_at_ms <= now {
                let ended = if core.ends_at_ms == 0 { now } else { core.ends_at_ms };
                let started = core
                    .session_started_at_ms
                    .unwrap_or(ended.saturating_sub(core.planned_ms));
                let session = NewSession {
                    planned_seconds: core.planned_ms / 1000,
                    actual_seconds: core.planned_ms / 1000,
                    started_at_ms: started as i64,
                    ended_at_ms: ended as i64,
                    status: "completed",
                };
                let _ = store.insert_session(&session);
                core.status = TimerStatus::Finished;
                core.remaining_ms = 0;
                core.ends_at_ms = ended;
                core.session_started_at_ms = None;
            } else if core.status == TimerStatus::Paused && core.remaining_ms == 0 {
                core.status = TimerStatus::Finished;
            }
        }
    }

    core
}

pub fn spawn_ticker(engine: Arc<TimerEngine>) {
    std::thread::spawn(move || {
        let mut last_second = 0u64;
        let mut last_persist = 0u64;
        loop {
            std::thread::sleep(Duration::from_millis(200));
            let now = now_ms();
            engine.finish_if_due(now);
            if now / 1000 != last_second {
                last_second = now / 1000;
                engine.emit_update();
                engine.update_tray();
            }
            if now.saturating_sub(last_persist) >= 5000 {
                last_persist = now;
                if engine.core.lock().status == TimerStatus::Running {
                    engine.persist();
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_down_and_finishes() {
        let mut timer = TimerCore::new(60_000);
        timer.start(1_000, 60_000);
        assert_eq!(timer.status, TimerStatus::Running);
        assert_eq!(timer.remaining_at(31_000), 30_000);
        assert!(!timer.tick(60_999));
        assert!(timer.tick(61_000));
        assert_eq!(timer.status, TimerStatus::Finished);
        assert_eq!(timer.remaining_at(61_000), 0);
    }

    #[test]
    fn pause_freezes_remaining_and_resume_rebases() {
        let mut timer = TimerCore::new(10_000);
        timer.start(1_000, 10_000);
        timer.pause(4_000);
        assert_eq!(timer.status, TimerStatus::Paused);
        assert_eq!(timer.remaining_ms, 7_000);
        assert_eq!(timer.remaining_at(1_000_000), 7_000);
        timer.resume(1_000_000);
        assert_eq!(timer.remaining_at(1_002_000), 5_000);
        assert_eq!(timer.elapsed_at(1_002_000), 5_000);
    }

    #[test]
    fn reset_restores_planned_duration() {
        let mut timer = TimerCore::new(25_000);
        timer.start(0, 25_000);
        timer.pause(5_000);
        timer.reset();
        assert_eq!(timer.status, TimerStatus::Idle);
        assert_eq!(timer.remaining_ms, 25_000);
        assert_eq!(timer.session_started_at_ms, None);
    }

    #[test]
    fn pause_at_deadline_marks_finished() {
        let mut timer = TimerCore::new(5_000);
        timer.start(0, 5_000);
        timer.pause(5_000);
        assert_eq!(timer.status, TimerStatus::Finished);
        assert_eq!(timer.remaining_ms, 0);
    }

    #[test]
    fn duration_is_never_below_one_second() {
        let mut timer = TimerCore::new(0);
        assert_eq!(timer.planned_ms, 1_000);
        timer.set_duration(0);
        assert_eq!(timer.planned_ms, 1_000);
    }

    #[test]
    fn clock_formatting() {
        assert_eq!(format_clock(0), "00:00");
        assert_eq!(format_clock(59_001), "01:00");
        assert_eq!(format_clock(3_600_000), "01:00:00");
    }

    #[test]
    fn elapsed_formatting_floors_without_rounding_up() {
        assert_eq!(format_elapsed(59_999), "00:59");
        assert_eq!(format_elapsed(3_661_000), "01:01:01");
    }

    #[test]
    fn stopwatch_counts_up_and_survives_pause() {
        let mut timer = TimerCore::new(25_000);
        timer.set_mode(TimerMode::Stopwatch);
        timer.start_stopwatch(10_000);
        assert_eq!(timer.elapsed_at(12_345), 2_345);
        assert_eq!(timer.remaining_at(12_345), 0);
        timer.pause_stopwatch(15_000);
        assert_eq!(timer.status, TimerStatus::Paused);
        assert_eq!(timer.stopwatch_base_ms, 5_000);
        assert_eq!(timer.elapsed_at(90_000), 5_000);
        timer.resume_stopwatch(90_000);
        assert_eq!(timer.elapsed_at(91_500), 6_500);
        assert!(!timer.tick(999_999));
        assert_eq!(timer.status, TimerStatus::Running);
    }

    #[test]
    fn stopwatch_snapshot_has_no_countdown_fields() {
        let mut timer = TimerCore::new(25_000);
        timer.set_mode(TimerMode::Stopwatch);
        timer.start_stopwatch(1_000);
        let snapshot = timer.snapshot(6_000);
        assert_eq!(snapshot.mode, TimerMode::Stopwatch);
        assert_eq!(snapshot.elapsed_ms, 5_000);
        assert_eq!(snapshot.remaining_ms, 0);
        assert_eq!(snapshot.progress, 0.0);
        assert_eq!(snapshot.ends_at_ms, None);
        assert_eq!(snapshot.planned_ms, 25_000);
    }

    #[test]
    fn switching_modes_clears_the_current_run() {
        let mut timer = TimerCore::new(25_000);
        timer.start(1_000, 25_000);
        timer.set_mode(TimerMode::Stopwatch);
        assert_eq!(timer.status, TimerStatus::Idle);
        assert_eq!(timer.mode, TimerMode::Stopwatch);
        assert_eq!(timer.session_started_at_ms, None);
        assert_eq!(timer.stopwatch_base_ms, 0);
        assert_eq!(timer.planned_ms, 25_000);
    }

    #[test]
    fn legacy_timer_state_defaults_to_countdown() {
        let legacy = r#"{"status":"idle","planned_ms":1500000,"remaining_ms":1500000,"ends_at_ms":0,"session_started_at_ms":null}"#;
        let timer: TimerCore = serde_json::from_str(legacy).expect("legacy timer parses");
        assert_eq!(timer.mode, TimerMode::Countdown);
        assert_eq!(timer.stopwatch_base_ms, 0);
        assert_eq!(timer.stopwatch_started_at_ms, 0);
    }
}
