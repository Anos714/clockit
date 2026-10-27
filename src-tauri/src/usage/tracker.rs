use std::collections::HashMap;
use std::os::fd::AsRawFd;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use wayland_backend::rs::client::ObjectId;
use wayland_client::backend::WaylandError;
use wayland_client::event_created_child;
use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::{
    self as toplevel_handle, ExtForeignToplevelHandleV1,
};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_list_v1::{
    self as toplevel_list, ExtForeignToplevelListV1,
};
use wayland_protocols::ext::idle_notify::v1::client::ext_idle_notification_v1::{
    self as idle_notification, ExtIdleNotificationV1,
};
use wayland_protocols::ext::idle_notify::v1::client::ext_idle_notifier_v1::ExtIdleNotifierV1;

use crate::store::Store;
use crate::timer::now_ms;
use crate::usage::protocol::cosmic::toplevel::zcosmic_toplevel_handle_v1::{
    self as cosmic_handle, ZcosmicToplevelHandleV1,
};
use crate::usage::protocol::cosmic::toplevel::zcosmic_toplevel_info_v1::ZcosmicToplevelInfoV1;
use crate::usage::{Control, PauseReason, UsageStatus};

const CHECKPOINT_MS: u64 = 30_000;
const MIN_SEGMENT_MS: u64 = 1_000;
const RECONNECT_MS: u64 = 15_000;
const LOCK_POLL_MS: u64 = 1_000;

pub(super) fn run(
    store: Arc<Store>,
    status: Arc<Mutex<UsageStatus>>,
    mut enabled: bool,
    mut idle_ms: u64,
    rx: Receiver<Control>,
) {
    let mut session: Option<Session> = None;
    let mut lock = LockWatcher::new();
    let mut next_connect = 0u64;
    let mut last_lock_poll = 0u64;

    loop {
        while let Ok(control) = rx.try_recv() {
            match control {
                Control::Configure {
                    enabled: new_enabled,
                    idle_ms: new_idle,
                } => {
                    let now = now_ms();
                    if let Some(current) = session.as_mut() {
                        if new_idle != idle_ms {
                            let qh = current.qh.clone();
                            current.state.set_idle_timeout(new_idle, &qh, now);
                        }
                        if new_enabled {
                            current.state.set_enabled(true, now);
                        }
                    }
                    let was_enabled = enabled;
                    enabled = new_enabled;
                    idle_ms = new_idle;
                    if !enabled {
                        if let Some(current) = session.take() {
                            current.finish(now, &store, PauseReason::Disabled);
                        }
                        publish_disabled(&status);
                    } else if !was_enabled {
                        next_connect = 0;
                    }
                }
                Control::Clear(ack) => {
                    let now = now_ms();
                    if let Some(current) = session.as_mut() {
                        current.state.recorder.clear(now);
                    }
                    let _ = ack.send(store.clear_usage());
                }
                Control::Shutdown(ack) => {
                    if let Some(current) = session.take() {
                        current.finish(now_ms(), &store, PauseReason::Disabled);
                    }
                    let _ = ack.send(());
                    return;
                }
            }
        }

        if !enabled {
            publish_disabled(&status);
            std::thread::sleep(Duration::from_millis(200));
            continue;
        }

        if session.is_none() {
            let now = now_ms();
            if now >= next_connect {
                match Session::connect(store.clone(), status.clone(), idle_ms) {
                    Ok(opened) => session = Some(opened),
                    Err(reason) => {
                        eprintln!("usage tracker unavailable: {reason}");
                        publish_unavailable(&status, &reason, true);
                        next_connect = now + RECONNECT_MS;
                    }
                }
            }
            if session.is_none() {
                std::thread::sleep(Duration::from_millis(250));
                continue;
            }
            last_lock_poll = 0;
        }

        let now = now_ms();
        if now.saturating_sub(last_lock_poll) >= LOCK_POLL_MS {
            last_lock_poll = now;
            let locked = lock.poll(now);
            if let Some(current) = session.as_mut() {
                if current.state.locked != locked {
                    current.state.locked = locked;
                    current.state.apply_pause(now);
                }
            }
        }

        let dispatched = session
            .as_mut()
            .expect("session checked above")
            .dispatch(Duration::from_millis(200));
        if let Err(error) = dispatched {
            publish_unavailable(&status, &error, true);
            session = None;
            next_connect = now_ms() + RECONNECT_MS;
            continue;
        }

        let now = now_ms();
        if let Some(current) = session.as_mut() {
            current.state.sync_active(now);
            current.state.recorder.checkpoint(now, &store);
            current.state.publish(true, None, lock.supported());
        }
    }
}

fn publish_disabled(status: &Mutex<UsageStatus>) {
    let mut status = status.lock();
    *status = UsageStatus {
        tracking: false,
        paused: true,
        pause_reason: Some(PauseReason::Disabled),
        ..UsageStatus::default()
    };
}

fn publish_unavailable(status: &Mutex<UsageStatus>, reason: &str, enabled: bool) {
    let mut status = status.lock();
    *status = UsageStatus {
        reason: Some(reason.to_string()),
        tracking: enabled,
        paused: enabled,
        lock_supported: false,
        ..UsageStatus::default()
    };
}

struct Session {
    conn: Connection,
    queue: EventQueue<State>,
    qh: QueueHandle<State>,
    state: State,
}

impl Session {
    fn connect(
        store: Arc<Store>,
        status: Arc<Mutex<UsageStatus>>,
        idle_ms: u64,
    ) -> Result<Self, String> {
        let conn = Connection::connect_to_env()
            .map_err(|_| "Wayland session not detected".to_string())?;
        let mut queue = conn.new_event_queue();
        let qh = queue.handle();
        conn.display().get_registry(&qh, ());

        let mut state = State::new(store, status, idle_ms);
        queue
            .roundtrip(&mut state)
            .map_err(|error| format!("Wayland setup failed: {error}"))?;

        if state.info.is_none() || state.list.is_none() {
            return Err("This session does not expose COSMIC window information".to_string());
        }
        state.subscribe_idle(&qh);
        conn.flush()
            .map_err(|error| format!("Wayland flush failed: {error}"))?;

        Ok(Self {
            conn,
            queue,
            qh,
            state,
        })
    }

    fn dispatch(&mut self, timeout: Duration) -> Result<(), String> {
        if let Some(guard) = self.conn.prepare_read() {
            let mut descriptor = libc::pollfd {
                fd: guard.connection_fd().as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            let timeout = timeout.as_millis().min(i32::MAX as u128) as i32;
            let ready = unsafe { libc::poll(&mut descriptor, 1, timeout) };
            if ready > 0 {
                match guard.read() {
                    Ok(_) => {}
                    Err(WaylandError::Io(error))
                        if error.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(error) => {
                        return Err(format!("Wayland connection lost: {error}"));
                    }
                }
            }
        }
        self.queue
            .dispatch_pending(&mut self.state)
            .map_err(|error| format!("Wayland dispatch failed: {error}"))?;
        self.conn
            .flush()
            .map_err(|error| format!("Wayland flush failed: {error}"))
    }

    fn finish(mut self, now: u64, store: &Store, reason: PauseReason) {
        self.state.recorder.pause_at(reason, now, store);
    }
}

#[derive(Default)]
struct Top {
    app_id: Option<String>,
    activated: bool,
    _cosmic: Option<ZcosmicToplevelHandleV1>,
}

struct State {
    store: Arc<Store>,
    status: Arc<Mutex<UsageStatus>>,
    recorder: Recorder,
    toplevels: HashMap<ObjectId, Top>,
    active: Option<ObjectId>,
    info: Option<ZcosmicToplevelInfoV1>,
    list: Option<ExtForeignToplevelListV1>,
    idle_notifier: Option<ExtIdleNotifierV1>,
    idle_notification: Option<ExtIdleNotificationV1>,
    seat: Option<wl_seat::WlSeat>,
    idle_ms: u64,
    enabled: bool,
    idle: bool,
    locked: bool,
    idle_supported: bool,
}

impl State {
    fn new(store: Arc<Store>, status: Arc<Mutex<UsageStatus>>, idle_ms: u64) -> Self {
        Self {
            store,
            status,
            recorder: Recorder::default(),
            toplevels: HashMap::new(),
            active: None,
            info: None,
            list: None,
            idle_notifier: None,
            idle_notification: None,
            seat: None,
            idle_ms,
            enabled: true,
            idle: false,
            locked: false,
            idle_supported: false,
        }
    }

    fn set_enabled(&mut self, enabled: bool, now: u64) {
        if self.enabled == enabled {
            return;
        }
        self.enabled = enabled;
        if !enabled {
            self.recorder.active_app = None;
        }
        self.apply_pause(now);
    }

    fn set_idle_timeout(&mut self, idle_ms: u64, qh: &QueueHandle<Self>, now: u64) {
        if self.idle_ms == idle_ms {
            return;
        }
        self.idle_ms = idle_ms;
        if let Some(notification) = self.idle_notification.take() {
            notification.destroy();
        }
        self.subscribe_idle(qh);
        self.idle = false;
        self.apply_pause(now);
    }

    fn subscribe_idle(&mut self, qh: &QueueHandle<Self>) {
        let (Some(notifier), Some(seat)) = (self.idle_notifier.clone(), self.seat.clone()) else {
            self.idle_supported = false;
            return;
        };
        let timeout = self.idle_ms.min(u32::MAX as u64) as u32;
        self.idle_notification =
            Some(notifier.get_idle_notification(timeout, &seat, qh, ()));
        self.idle_supported = true;
    }

    fn apply_pause(&mut self, now: u64) {
        let reason = if !self.enabled {
            Some(PauseReason::Disabled)
        } else if self.locked {
            Some(PauseReason::Locked)
        } else if self.idle {
            Some(PauseReason::Idle)
        } else {
            None
        };

        match reason {
            Some(PauseReason::Idle) => {
                let started = now.saturating_sub(self.idle_ms);
                self.recorder.pause_at(PauseReason::Idle, started, &self.store);
            }
            Some(reason) => self.recorder.pause_at(reason, now, &self.store),
            None => self.recorder.resume(now),
        }
    }

    fn sync_active(&mut self, now: u64) {
        let app = self
            .active
            .as_ref()
            .and_then(|id| self.toplevels.get(id))
            .and_then(|top| top.app_id.clone());
        self.recorder.set_active(app, now, &self.store);
    }

    fn recompute_active(&mut self) {
        if let Some(id) = self.active.as_ref() {
            if self.toplevels.get(id).is_some_and(|top| top.activated) {
                return;
            }
        }
        self.active = self
            .toplevels
            .iter()
            .find(|(_, top)| top.activated)
            .map(|(id, _)| id.clone());
    }

    fn publish(&self, available: bool, reason: Option<&str>, lock_supported: bool) {
        let mut status = self.status.lock();
        *status = UsageStatus {
            available,
            reason: reason.map(str::to_string),
            tracking: self.enabled,
            paused: self.recorder.paused.is_some(),
            pause_reason: self.recorder.paused,
            active_app: self.recorder.active_app.clone(),
            active_since_ms: self.recorder.open.as_ref().map(|open| open.started_at_ms as i64),
            idle: self.idle,
            locked: self.locked,
            idle_supported: self.idle_supported,
            lock_supported,
        };
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "zcosmic_toplevel_info_v1" if state.info.is_none() => {
                    state.info = Some(registry.bind::<ZcosmicToplevelInfoV1, _, _>(
                        name,
                        version.min(3),
                        qh,
                        (),
                    ));
                }
                "ext_foreign_toplevel_list_v1" if state.list.is_none() => {
                    state.list = Some(registry.bind::<ExtForeignToplevelListV1, _, _>(
                        name,
                        version.min(1),
                        qh,
                        (),
                    ));
                }
                "ext_idle_notifier_v1" if state.idle_notifier.is_none() => {
                    state.idle_notifier = Some(registry.bind::<ExtIdleNotifierV1, _, _>(
                        name,
                        version.min(1),
                        qh,
                        (),
                    ));
                }
                "wl_seat" if state.seat.is_none() => {
                    state.seat = Some(registry.bind::<wl_seat::WlSeat, _, _>(
                        name,
                        version.min(1),
                        qh,
                        (),
                    ));
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<ExtForeignToplevelListV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ExtForeignToplevelListV1,
        event: toplevel_list::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let toplevel_list::Event::Toplevel { toplevel } = event {
            let id = toplevel.id();
            state.toplevels.insert(id.clone(), Top::default());
            if let Some(info) = state.info.clone() {
                let cosmic = info.get_cosmic_toplevel(&toplevel, qh, ());
                if let Some(top) = state.toplevels.get_mut(&id) {
                    top._cosmic = Some(cosmic);
                }
            }
        }
    }

    event_created_child!(State, ExtForeignToplevelListV1, [
        toplevel_list::EVT_TOPLEVEL_OPCODE => (ExtForeignToplevelHandleV1, ()),
    ]);
}

impl Dispatch<ExtForeignToplevelHandleV1, ()> for State {
    fn event(
        state: &mut Self,
        handle: &ExtForeignToplevelHandleV1,
        event: toplevel_handle::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            toplevel_handle::Event::AppId { app_id } => {
                if let Some(top) = state.toplevels.get_mut(&handle.id()) {
                    top.app_id = Some(app_id.trim().to_string());
                }
            }
            toplevel_handle::Event::Closed => {
                let id = handle.id();
                if let Some(top) = state.toplevels.remove(&id) {
                    if let Some(cosmic) = top._cosmic {
                        cosmic.destroy();
                    }
                }
                handle.destroy();
                state.recompute_active();
            }
            _ => {}
        }
    }
}

impl Dispatch<ZcosmicToplevelInfoV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &ZcosmicToplevelInfoV1,
        _: <ZcosmicToplevelInfoV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZcosmicToplevelHandleV1, ()> for State {
    fn event(
        state: &mut Self,
        handle: &ZcosmicToplevelHandleV1,
        event: cosmic_handle::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let cosmic_handle::Event::State { state: states } = event {
            let cosmic_id = handle.id();
            let Some(ext_id) = state
                .toplevels
                .iter()
                .find(|(_, top)| {
                    top._cosmic
                        .as_ref()
                        .is_some_and(|cosmic| cosmic.id() == cosmic_id)
                })
                .map(|(id, _)| id.clone())
            else {
                return;
            };
            let activated = states.iter().any(|value| *value == 2);
            if let Some(top) = state.toplevels.get_mut(&ext_id) {
                top.activated = activated;
            }
            if activated {
                state.active = Some(ext_id);
            } else if state.active.as_ref() == Some(&ext_id) {
                state.recompute_active();
            }
        }
    }
}

impl Dispatch<ExtIdleNotifierV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &ExtIdleNotifierV1,
        _: <ExtIdleNotifierV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtIdleNotificationV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ExtIdleNotificationV1,
        event: idle_notification::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let now = now_ms();
        match event {
            idle_notification::Event::Idled => {
                state.idle = true;
                state.apply_pause(now);
            }
            idle_notification::Event::Resumed => {
                state.idle = false;
                state.apply_pause(now);
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn event(
        _: &mut Self,
        _: &wl_seat::WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

struct OpenSegment {
    app_id: String,
    started_at_ms: u64,
}

#[derive(Default)]
struct Recorder {
    open: Option<OpenSegment>,
    active_app: Option<String>,
    paused: Option<PauseReason>,
}

impl Recorder {
    fn set_active(&mut self, app: Option<String>, now: u64, store: &Store) {
        if self.active_app == app {
            return;
        }
        self.active_app = app;
        if self.paused.is_none() {
            self.switch(now, store, MIN_SEGMENT_MS);
        }
    }

    fn switch(&mut self, now: u64, store: &Store, min_ms: u64) {
        self.flush(now, store, min_ms);
        self.open_if_active(now);
    }

    fn open_if_active(&mut self, now: u64) {
        if self.paused.is_some() {
            return;
        }
        if let Some(app_id) = self.active_app.clone() {
            self.open = Some(OpenSegment {
                app_id,
                started_at_ms: now,
            });
        }
    }

    fn flush(&mut self, now: u64, store: &Store, min_ms: u64) {
        if let Some(open) = self.open.take() {
            let ended = now.max(open.started_at_ms);
            if ended.saturating_sub(open.started_at_ms) >= min_ms {
                if let Err(error) = store.insert_usage(&open.app_id, open.started_at_ms, ended) {
                    eprintln!("could not record app usage: {error}");
                }
            }
        }
    }

    fn pause_at(&mut self, reason: PauseReason, at_ms: u64, store: &Store) {
        if self.paused == Some(reason) {
            return;
        }
        self.paused = Some(reason);
        let ended = at_ms.max(self.open.as_ref().map(|open| open.started_at_ms).unwrap_or(at_ms));
        self.flush(ended, store, 0);
    }

    fn resume(&mut self, now: u64) {
        if self.paused.is_none() {
            return;
        }
        self.paused = None;
        if self.open.is_none() {
            self.open_if_active(now);
        }
    }

    fn checkpoint(&mut self, now: u64, store: &Store) {
        if self.paused.is_some() {
            return;
        }
        let due = self
            .open
            .as_ref()
            .is_some_and(|open| now.saturating_sub(open.started_at_ms) >= CHECKPOINT_MS);
        if due {
            self.switch(now, store, 0);
        }
    }

    fn clear(&mut self, now: u64) {
        self.open = None;
        self.open_if_active(now);
    }

    #[cfg(test)]
    fn shutdown(&mut self, now: u64, store: &Store) {
        self.flush(now, store, 0);
    }
}

struct LockWatcher {
    proxy: Option<zbus::blocking::Proxy<'static>>,
    supported: bool,
    locked: bool,
    next_attempt_ms: u64,
}

impl LockWatcher {
    fn new() -> Self {
        Self {
            proxy: None,
            supported: false,
            locked: false,
            next_attempt_ms: 0,
        }
    }

    fn supported(&self) -> bool {
        self.supported
    }

    fn poll(&mut self, now: u64) -> bool {
        if self.proxy.is_none() {
            if now < self.next_attempt_ms {
                return self.locked;
            }
            self.next_attempt_ms = now + 30_000;
            if let Ok(connection) = zbus::blocking::Connection::system() {
                if let Ok(proxy) = zbus::blocking::Proxy::new_owned(
                    connection,
                    "org.freedesktop.login1",
                    "/org/freedesktop/login1/session/auto",
                    "org.freedesktop.login1.Session",
                ) {
                    self.proxy = Some(proxy);
                }
            }
            if self.proxy.is_none() {
                self.supported = false;
                return self.locked;
            }
        }

        let Some(proxy) = self.proxy.as_ref() else {
            return self.locked;
        };
        match proxy.get_property::<bool>("LockedHint") {
            Ok(locked) => {
                self.locked = locked;
                self.supported = true;
                locked
            }
            Err(_) => {
                self.proxy = None;
                self.supported = false;
                self.next_attempt_ms = now + 30_000;
                self.locked
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_store(name: &str) -> (Store, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "clockit-usage-test-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        (Store::open(&dir).expect("store opens"), dir)
    }

    #[test]
    fn recorder_splits_on_app_switch_and_ignores_flickers() {
        let (store, dir) = temp_store("recorder");
        let mut recorder = Recorder::default();

        recorder.set_active(Some("editor".into()), 1_000, &store);
        recorder.set_active(Some("browser".into()), 5_000, &store);
        // A sub-second flicker is dropped instead of written.
        recorder.set_active(Some("terminal".into()), 5_400, &store);
        recorder.set_active(Some("browser".into()), 5_800, &store);
        recorder.shutdown(10_000, &store);

        let apps = store.usage_by_app("1900-01-01", 5).unwrap();
        assert_eq!(apps.len(), 2);
        let browser = apps
            .iter()
            .find(|app| app.app_id == "browser")
            .expect("browser recorded");
        assert_eq!(browser.total_ms, 4_200);
        let editor = apps
            .iter()
            .find(|app| app.app_id == "editor")
            .expect("editor recorded");
        assert_eq!(editor.total_ms, 4_000);

        drop(store);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn recursion_pause_trims_idle_time_and_resumes() {
        let (store, dir) = temp_store("idle");
        let mut recorder = Recorder::default();

        recorder.set_active(Some("writer".into()), 10_000, &store);
        // Idle began at 40s, the notification arrived at 70s with a 30s timeout.
        recorder.pause_at(PauseReason::Idle, 20_000, &store);
        assert_eq!(recorder.paused, Some(PauseReason::Idle));
        assert!(recorder.open.is_none());

        recorder.resume(80_000);
        assert_eq!(recorder.paused, None);
        recorder.shutdown(90_000, &store);

        let apps = store.usage_by_app("1900-01-01", 5).unwrap();
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].app_id, "writer");
        assert_eq!(apps[0].total_ms, 20_000);

        drop(store);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn checkpoint_writes_and_keeps_recording() {
        let (store, dir) = temp_store("checkpoint");
        let mut recorder = Recorder::default();

        recorder.set_active(Some("music".into()), 0, &store);
        recorder.checkpoint(CHECKPOINT_MS, &store);
        assert!(recorder.open.is_some());
        recorder.checkpoint(CHECKPOINT_MS + 500, &store);
        recorder.shutdown(CHECKPOINT_MS + 500, &store);

        let apps = store.usage_by_app("1900-01-01", 5).unwrap();
        assert_eq!(apps[0].total_ms, CHECKPOINT_MS + 500);

        drop(store);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn clear_drops_the_open_segment() {
        let (store, dir) = temp_store("clear");
        let mut recorder = Recorder::default();

        recorder.set_active(Some("editor".into()), 1_000, &store);
        recorder.clear(9_000);
        recorder.shutdown(9_500, &store);

        assert_eq!(store.usage_total("1900-01-01").unwrap(), 500);

        drop(store);
        let _ = std::fs::remove_dir_all(dir);
    }
}
