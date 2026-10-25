use tauri::{AppHandle, Manager};

use crate::timer::TimerMode;
use crate::windows;
use crate::AppState;

#[derive(Debug, Default)]
pub struct Cli {
    hidden: bool,
    quit: bool,
    show: bool,
    mini: bool,
    pause: bool,
    resume: bool,
    reset: bool,
    toggle: bool,
    stopwatch: bool,
    countdown: bool,
    start: Option<Option<u64>>,
}

impl Cli {
    fn has_action(&self) -> bool {
        self.quit
            || self.mini
            || self.pause
            || self.resume
            || self.reset
            || self.toggle
            || self.stopwatch
            || self.countdown
            || self.start.is_some()
    }
}

fn parse(argv: &[String]) -> Cli {
    let mut cli = Cli::default();
    let mut index = 1;

    while index < argv.len() {
        let arg = argv[index].as_str();
        match arg {
            "--hidden" => cli.hidden = true,
            "--quit" => cli.quit = true,
            "--show" => cli.show = true,
            "--mini" => cli.mini = true,
            "--pause" => cli.pause = true,
            "--resume" => cli.resume = true,
            "--reset" => cli.reset = true,
            "--toggle" => cli.toggle = true,
            "--stopwatch" => cli.stopwatch = true,
            "--countdown" => cli.countdown = true,
            "--start" => {
                let next = argv.get(index + 1).and_then(|value| value.parse::<u64>().ok());
                if next.is_some() {
                    index += 1;
                }
                cli.start = Some(next);
            }
            _ => {
                if let Some(value) = arg.strip_prefix("--start=") {
                    cli.start = Some(value.parse::<u64>().ok());
                }
            }
        }
        index += 1;
    }

    cli
}

pub fn handle(app: &AppHandle, argv: &[String]) {
    let cli = parse(argv);

    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let engine = &state.engine;

    if cli.quit {
        engine.quitting.store(true, std::sync::atomic::Ordering::SeqCst);
        app.exit(0);
        return;
    }

    if cli.stopwatch {
        engine.set_mode(TimerMode::Stopwatch);
    }
    if cli.countdown {
        engine.set_mode(TimerMode::Countdown);
    }
    if let Some(seconds) = cli.start {
        engine.start(seconds);
    }
    if cli.pause {
        engine.pause();
    }
    if cli.resume {
        engine.resume();
    }
    if cli.reset {
        engine.reset();
    }
    if cli.toggle {
        engine.toggle();
    }
    if cli.mini {
        let _ = windows::open_mini(app);
    }

    if cli.show {
        windows::show_main(app);
    } else if !cli.hidden && !cli.has_action() {
        windows::show_main(app);
    }
}
