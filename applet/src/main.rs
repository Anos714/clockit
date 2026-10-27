use std::process::{Command, Stdio};
use std::time::Duration;

use cosmic::app::{Core, Task};
use cosmic::iced::core::window::Id;
use cosmic::iced::core::Alignment;
use cosmic::iced::futures::SinkExt;
use cosmic::iced::widget::{column, row};
use cosmic::iced::{Length, Subscription};
use cosmic::theme::Button;
use cosmic::widget::{button, icon};
use cosmic::Element;

#[zbus::proxy(
    interface = "com.clockit.Timer",
    default_service = "com.clockit.Timer",
    default_path = "/com/clockit/Timer"
)]
trait Timer {
    fn state(&self) -> zbus::Result<(String, String, u64, u64, u64)>;
    fn toggle(&self) -> zbus::Result<()>;
}

const APP_ID: &str = "com.clockit.Clockit";
const POLL_MS: u64 = 500;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct TimerState {
    online: bool,
    mode: String,
    status: String,
    planned_ms: u64,
    remaining_ms: u64,
    elapsed_ms: u64,
}

impl TimerState {
    fn running(&self) -> bool {
        self.status == "running"
    }

    fn label(&self) -> String {
        if !self.online {
            return "—".to_string();
        }
        if self.mode == "stopwatch" {
            return format_clock(self.elapsed_ms, false);
        }
        let ms = match self.status.as_str() {
            "idle" => self.planned_ms,
            "running" | "paused" => self.remaining_ms,
            _ => 0,
        };
        format_clock(ms, true)
    }

    fn icon(&self) -> &'static str {
        if self.running() {
            "media-playback-pause-symbolic"
        } else if self.online && self.status == "finished" {
            "alarm-symbolic"
        } else {
            "media-playback-start-symbolic"
        }
    }

    fn tooltip(&self) -> &'static str {
        if !self.online {
            "Clockit is not running · click to open"
        } else {
            match self.status.as_str() {
                "running" => "Running · click to pause",
                "paused" => "Paused · click to resume",
                "finished" => "Finished · click to start again",
                _ => "Click to start",
            }
        }
    }
}

fn format_clock(ms: u64, round_up: bool) -> String {
    let seconds = if round_up {
        ms.saturating_add(999) / 1000
    } else {
        ms / 1000
    };
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

fn spawn_toggle() -> bool {
    for program in ["clockit", "/usr/bin/clockit"] {
        let spawned = Command::new(program)
            .arg("--toggle")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        if spawned.is_ok() {
            return true;
        }
    }
    false
}

async fn toggle_timer() -> bool {
    let connection = match zbus::Connection::session().await {
        Ok(connection) => connection,
        Err(_) => return spawn_toggle(),
    };
    let proxy = match TimerProxy::new(&connection).await {
        Ok(proxy) => proxy,
        Err(_) => return spawn_toggle(),
    };
    match proxy.toggle().await {
        Ok(()) => true,
        Err(_) => spawn_toggle(),
    }
}

fn timer_stream() -> Subscription<Message> {
    Subscription::run(|| {
        cosmic::iced::stream::channel(4, async |mut output| {
            let mut connection: Option<zbus::Connection> = None;
            loop {
                if connection.is_none() {
                    connection = zbus::Connection::session().await.ok();
                }

                let mut update = TimerState::default();
                if let Some(current) = connection.as_ref() {
                    match TimerProxy::new(current).await {
                        Ok(proxy) => match proxy.state().await {
                            Ok((mode, status, planned_ms, remaining_ms, elapsed_ms)) => {
                                update = TimerState {
                                    online: true,
                                    mode,
                                    status,
                                    planned_ms,
                                    remaining_ms,
                                    elapsed_ms,
                                };
                            }
                            Err(_) => connection = None,
                        },
                        Err(_) => connection = None,
                    }
                }

                if output.send(Message::Tick(update)).await.is_err() {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(POLL_MS)).await;
            }
        })
    })
}

struct Applet {
    core: Core,
    timer: TimerState,
}

#[derive(Clone, Debug)]
enum Message {
    Tick(TimerState),
    Toggle,
    Toggled,
    Surface(cosmic::surface::Action<Message>),
}

impl cosmic::Application for Applet {
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: ()) -> (Self, Task<Message>) {
        (
            Self {
                core,
                timer: TimerState::default(),
            },
            Task::none(),
        )
    }

    fn subscription(&self) -> Subscription<Message> {
        timer_stream()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick(update) => self.timer = update,
            Message::Toggle => {
                return cosmic::task::future(async {
                    let _ = toggle_timer().await;
                    Message::Toggled
                });
            }
            Message::Toggled => {}
            Message::Surface(action) => {
                return cosmic::task::message(cosmic::Action::Surface(action));
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let suggested = self.core.applet.suggested_size(true);
        let padding = self.core.applet.suggested_padding(true);
        let horizontal = self.core.applet.is_horizontal();

        let mut children: Vec<Element<Message>> = vec![
            icon::from_name(self.timer.icon())
                .symbolic(true)
                .size(suggested.0)
                .into(),
        ];
        children.push(
            self.core
                .applet
                .text(self.timer.label())
                .height(Length::Fixed(f32::from(suggested.1)))
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .into(),
        );

        let content: Element<Message> = if horizontal {
            row(children)
                .spacing(padding.1)
                .align_y(Alignment::Center)
                .into()
        } else {
            column(children)
                .spacing(padding.1)
                .align_x(Alignment::Center)
                .into()
        };

        let (horizontal_padding, vertical_padding) = if horizontal {
            (padding.0, padding.1)
        } else {
            (padding.1, padding.0)
        };

        let control: Element<Message> = button::custom(content)
            .on_press_down(Message::Toggle)
            .class(Button::AppletIcon)
            .padding([vertical_padding, horizontal_padding])
            .into();

        let tooltip = self.core.applet.applet_tooltip(
            control,
            self.timer.tooltip(),
            false,
            Message::Surface,
            None,
        );

        self.core.applet.autosize_window(tooltip).into()
    }

    fn view_window(&self, _id: Id) -> Element<'_, Message> {
        Element::from("")
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<Applet>(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(mode: &str, status: &str, planned_ms: u64, remaining_ms: u64, elapsed_ms: u64) -> TimerState {
        TimerState {
            online: true,
            mode: mode.to_string(),
            status: status.to_string(),
            planned_ms,
            remaining_ms,
            elapsed_ms,
        }
    }

    #[test]
    fn clock_rounds_remaining_time_up() {
        assert_eq!(format_clock(0, true), "0:00");
        assert_eq!(format_clock(59_001, true), "1:00");
        assert_eq!(format_clock(1_500_000, true), "25:00");
        assert_eq!(format_clock(3_600_000, true), "1:00:00");
    }

    #[test]
    fn clock_rounds_elapsed_time_down() {
        assert_eq!(format_clock(59_999, false), "0:59");
        assert_eq!(format_clock(65_400, false), "1:05");
    }

    #[test]
    fn idle_countdown_shows_planned_duration() {
        let idle = state("countdown", "idle", 1_500_000, 1_500_000, 0);
        assert_eq!(idle.label(), "25:00");
    }

    #[test]
    fn running_countdown_shows_remaining_time() {
        let running = state("countdown", "running", 1_500_000, 1_499_000, 1_000);
        assert_eq!(running.label(), "24:59");
        assert!(running.running());
    }

    #[test]
    fn stopwatch_shows_elapsed_time() {
        let stopwatch = state("stopwatch", "running", 0, 0, 65_400);
        assert_eq!(stopwatch.label(), "1:05");
    }

    #[test]
    fn offline_state_shows_placeholder() {
        let offline = TimerState::default();
        assert_eq!(offline.label(), "—");
        assert_eq!(offline.tooltip(), "Clockit is not running · click to open");
    }
}

