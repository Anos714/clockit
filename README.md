# Clockit

A focused desktop timer for Pop!_OS / COSMIC, built with Tauri 2, Rust, and
React. It lives in the COSMIC panel as a tray icon, keeps an authoritative
wall-clock timer in Rust, and stores sessions locally in SQLite.

## Features

- Countdown timer with 5 / 15 / 25 / 45 / 60 minute presets and a custom duration.
- Clock shows seconds and a live millisecond fraction in timer, stopwatch, mini window, and focus mode.
- Stopwatch with millisecond display that keeps its elapsed time across pauses and restarts.
- Start, pause, resume, reset. Keyboard: `Space` start/pause, `R` reset, `F` focus mode, `I` insights.
- Wall-clock engine: survives app restarts and system suspend.
- COSMIC panel tray with live time, start/pause, reset, mini window, quit.
- Native COSMIC panel applet with the live countdown; one click starts, pauses,
  or resumes the timer straight from the panel.
- Frameless mini window for a compact timer on screen.
- Fullscreen focus mode with very large digits, for both timer and stopwatch.
- Desktop notification and chime on countdown completion.
- SQLite session history (focus sessions, stopwatch runs, and ended-early sessions).
- Quick checklist with add, rename, complete, and delete, stored locally.
- Local app-usage insights on COSMIC: which app is active, weekly and hourly
  charts, most-used apps, and recent activity, recorded from the
  `cosmic-toplevel-info` Wayland protocol.
- Idle- and lock-aware tracking: time away from the keyboard or behind a locked
  screen is trimmed instead of counted.
- Single instance with CLI control flags, useful for COSMIC custom shortcuts.
- Forest theme in daylight and night-wood palettes, with system, light, and dark
  modes plus an in-app theme switch.
- Calm botanical artwork, animated branches and leaves, and reduced-motion support.
- Local only: no network calls, no telemetry, no accounts. Usage insight stores
  app IDs and durations only; window titles are never stored, and history is kept
  for 90 days.

## Prerequisites

Pop!_OS 24.04 / Ubuntu 24.04 build dependencies:

```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf
```

Also required: Rust (stable) and Bun.

## Development

```bash
bun install
bun run tauri dev
```

The first Rust build downloads and compiles the Tauri stack, so it takes a few
minutes.

## Build and install

```bash
bun run build:deb
sudo apt install ./src-tauri/target/release/bundle/deb/Clockit_0.1.0_amd64.deb
```

`build:deb` compiles the native panel applet first, then Tauri builds the app and
packs both the applet binary and its desktop entry into the `.deb`.

The installed binaries are `clockit` and `clockit-applet`. Data lives in
`~/.local/share/clockit/clockit.db`.

App-usage tracking uses Wayland protocols provided by COSMIC. On non-COSMIC
sessions the Insights view explains that window information is unavailable and
the rest of Clockit keeps working; tracking retries quietly in the background.

## CLI

The app runs as a single instance. A second invocation forwards the command to
the running app and exits, which is how you can bind shortcuts in COSMIC
Settings without Wayland global-hotkey support.

```bash
clockit --toggle          # start / pause / resume
clockit --start 1500      # start a 25 minute timer
clockit --stopwatch       # switch to stopwatch mode
clockit --countdown       # switch back to countdown mode
clockit --stopwatch --start
clockit --pause
clockit --resume
clockit --reset
clockit --show
clockit --mini
clockit --hidden          # start in the tray without opening the window
clockit --quit
```

Mode switches are applied before `--start`, so `clockit --stopwatch --start`
starts counting up immediately.

Suggested COSMIC shortcut commands: `clockit --toggle`, `clockit --reset`.

## Panel behavior on COSMIC

Clockit ships a native libcosmic panel applet, installed with the `.deb`. After
installing, open the COSMIC panel settings and add the Clockit applet. The
applet shows the live countdown and toggles the timer with a single click;
when Clockit is not running it shows a dash and launches the app on click.

The applet talks to the running app through the session D-Bus service
`com.clockit.Timer` (`/com/clockit/Timer`), which exposes `State`, `Toggle`,
`Start`, `Pause`, `Reset`, and `Show`. The service is local to your session and
does not open a network port.

Tauri still also provides an AppIndicator/StatusNotifierItem tray. COSMIC shows
it in the panel as a menu (left-click opens the menu, a Linux appindicator
limitation), so use the native applet for the one-click control.

The mini window requests always-on-top, but Wayland compositors may ignore that
hint. COSMIC window rules are the reliable solution.

## Architecture

```
src-tauri/src/
  lib.rs        app wiring, plugins, window lifecycle
  timer.rs      wall-clock countdown + stopwatch engine, tray labels, alerts
  timer_bus.rs  session D-Bus service used by the panel applet
  store.rs      SQLite: settings, timer snapshot, sessions, tasks, usage
  settings.rs   persisted preferences, including theme and privacy
  tray.rs       tray icon and menu
  cli.rs        CLI flags + single-instance forwarding
  commands.rs   Tauri commands used by the UI
  windows.rs    main/mini window helpers
  usage/
    mod.rs      usage service, status, snapshots, retention
    tracker.rs  COSMIC toplevel + idle + logind lock tracker
    protocol.rs generated cosmic-toplevel-info bindings
src-tauri/protocols/  vendored COSMIC protocol XML (permissive licence)
src/
  App.tsx       main window + focus mode UI
  MiniApp.tsx   mini window UI
  useTimer.ts   backend state + smooth local clock
  useTheme.ts   system / light / dark theme sync
  theme.ts      theme resolution and flash-free caching
  format.ts     clock, duration, and insights formatting
  usage.ts      friendly app-name display for app IDs
  styles.css    forest design system, motion, responsive rules
  components/   TimerRing, ModeSwitch, TodoList, SessionList,
                Insights, SettingsPanel, Botanical, Icons
public/fonts/   bundled Fraunces + Karla variable fonts (OFL)
applet/
  src/main.rs   native libcosmic panel applet and D-Bus client
  data/         applet desktop entry, installed by the .deb
```

## Design notes

- Typography: Fraunces for editorial headings and Karla for UI and clock digits
  (tabular numerals). Both fonts are bundled, so the app makes no font requests.
- Motion: micro-interactions and screen transitions use Motion, while the
  botanical SVG branches, drifting leaves, and fireflies are CSS-animated.
  Entrances that carry essential content use CSS so they still paint when the
  window starts unfocused.
- The theme defaults to the desktop preference. On COSMIC the system value comes
  from the XDG appearance portal through Tauri, with `prefers-color-scheme` as a
  fallback. The last resolved theme is cached to avoid a flash on startup.
- Motion is set to respect the desktop's reduced-motion setting.

## Roadmap

- Phase 2 (complete): app usage tracking on COSMIC via the
  `cosmic-toplevel-info` Wayland protocol, idle/lock handling, and a local
  Insights view with privacy settings.
- Phase 3 (complete): native libcosmic panel applet over D-Bus with a one-click
  panel timer. The applet ships in the `.deb` and is added from the COSMIC panel
  settings.

## Notes

- The completion sound is played through `pw-play` or `paplay` with a bundled
  WAV, so no audio dev libraries are needed.
- Fraunces and Karla are bundled under the SIL Open Font License; license files
  are in `public/fonts/`.
- All artwork is inline SVG or CSS; no images are fetched at runtime.
- Autostart is opt-in from Settings. In `tauri dev` it registers the debug
  binary, which is not useful after development.
- Usage insights require a COSMIC Wayland session; Wayland prevents generic
  global window inspection. Only app IDs are stored, split at local hour and day
  boundaries, and old rows are pruned after 90 days.
