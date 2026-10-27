use std::sync::Arc;

use crate::timer::{TimerEngine, TimerMode, TimerStatus};
use crate::windows;

const SERVICE: &str = "com.clockit.Timer";
const PATH: &str = "/com/clockit/Timer";

struct TimerBus {
    engine: Arc<TimerEngine>,
}

#[zbus::interface(name = "com.clockit.Timer")]
impl TimerBus {
    #[zbus(name = "State")]
    fn state(&self) -> (String, String, u64, u64, u64) {
        let snapshot = self.engine.snapshot();
        (
            mode_name(snapshot.mode).to_string(),
            status_name(snapshot.status).to_string(),
            snapshot.planned_ms,
            snapshot.remaining_ms,
            snapshot.elapsed_ms,
        )
    }

    #[zbus(name = "Toggle")]
    fn toggle(&self) {
        self.engine.toggle();
    }

    #[zbus(name = "Start")]
    fn start(&self) {
        self.engine.start(None);
    }

    #[zbus(name = "Pause")]
    fn pause(&self) {
        self.engine.pause();
    }

    #[zbus(name = "Reset")]
    fn reset(&self) {
        self.engine.reset();
    }

    #[zbus(name = "Show")]
    fn show(&self) {
        windows::show_main(&self.engine.app);
    }
}

fn mode_name(mode: TimerMode) -> &'static str {
    match mode {
        TimerMode::Countdown => "countdown",
        TimerMode::Stopwatch => "stopwatch",
    }
}

fn status_name(status: TimerStatus) -> &'static str {
    match status {
        TimerStatus::Idle => "idle",
        TimerStatus::Running => "running",
        TimerStatus::Paused => "paused",
        TimerStatus::Finished => "finished",
    }
}

pub fn spawn(engine: Arc<TimerEngine>) {
    let bus = TimerBus { engine };
    std::thread::Builder::new()
        .name("clockit-dbus".to_string())
        .spawn(move || {
            let result = (|| -> zbus::Result<zbus::blocking::Connection> {
                let builder = zbus::blocking::connection::Builder::session()?
                    .name(SERVICE)?
                    .serve_at(PATH, bus)?;
                builder.build()
            })();
            let _connection = match result {
                Ok(connection) => connection,
                Err(error) => {
                    eprintln!("timer D-Bus service unavailable: {error}");
                    return;
                }
            };
            loop {
                std::thread::park();
            }
        })
        .ok();
}
