# Tempo

A focused desktop timer for Pop!_OS / COSMIC, built with Tauri 2, Rust, and
React. It lives in the COSMIC panel as a tray icon, keeps an authoritative
wall-clock timer in Rust, and stores sessions locally in SQLite.

## Features

- Countdown timer with 5 / 15 / 25 / 45 / 60 minute presets and a custom duration.
- Clock shows seconds and a live millisecond fraction in timer, stopwatch, mini window, and focus mode.
- Stopwatch with millisecond display that keeps its elapsed time across pauses and restarts.
- Start, pause, resume, reset. Keyboard: `Space` start/pause, `R` reset, `F` focus mode.
- Wall-clock engine: survives app restarts and system suspend.
- COSMIC panel tray with live time, start/pause, reset, mini window, quit.
- Frameless mini window for a compact timer on screen.
- Fullscreen focus mode with very large digits, for both timer and stopwatch.
- Desktop notification and chime on countdown completion.
- SQLite session history (focus sessions, stopwatch runs, and ended-early sessions).
- Quick checklist with add, rename, complete, and delete, stored locally.
- Single instance with CLI control flags, useful for COSMIC custom shortcuts.
- Forest theme in daylight and night-wood palettes, with system, light, and dark
  modes plus an in-app theme switch.
- Calm botanical artwork, animated branches and leaves, and reduced-motion support.
- Local only: no network calls, no telemetry, no accounts.

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
bun run tauri build --bundles deb
sudo apt install ./src-tauri/target/release/bundle/deb/Tempo_0.1.0_amd64.deb
```

The installed binary is `tempo`. Data lives in `~/.local/share/tempo/tempo.db`.

## CLI

The app runs as a single instance. A second invocation forwards the command to
the running app and exits, which is how you can bind shortcuts in COSMIC
Settings without Wayland global-hotkey support.

```bash
tempo --toggle          # start / pause / resume
tempo --start 1500      # start a 25 minute timer
tempo --stopwatch       # switch to stopwatch mode
tempo --countdown       # switch back to countdown mode
tempo --stopwatch --start
tempo --pause
tempo --resume
tempo --reset
tempo --show
tempo --mini
tempo --hidden          # start in the tray without opening the window
tempo --quit
```

Mode switches are applied before `--start`, so `tempo --stopwatch --start`
starts counting up immediately.

Suggested COSMIC shortcut commands: `tempo --toggle`, `tempo --reset`.

## Panel behavior on COSMIC

Tauri uses an AppIndicator/StatusNotifierItem tray. COSMIC shows it in the panel;
left-click opens the menu (Linux appindicator limitation), so the panel control is
a menu rather than a single clickable timer button.

The mini window requests always-on-top, but Wayland compositors may ignore that
hint. COSMIC window rules or a future native COSMIC applet are the reliable
solutions.

## Architecture

```
src-tauri/src/
  lib.rs        app wiring, plugins, window lifecycle
  timer.rs      wall-clock countdown + stopwatch engine, tray labels, alerts
  store.rs      SQLite: settings, timer snapshot, sessions, tasks
  settings.rs   persisted preferences, including theme
  tray.rs       tray icon and menu
  cli.rs        CLI flags + single-instance forwarding
  commands.rs   Tauri commands used by the UI
  windows.rs    main/mini window helpers
src/
  App.tsx       main window + focus mode UI
  MiniApp.tsx   mini window UI
  useTimer.ts   backend state + smooth local clock
  useTheme.ts   system / light / dark theme sync
  theme.ts      theme resolution and flash-free caching
  format.ts     clock and duration formatting
  styles.css    forest design system, motion, responsive rules
  components/   TimerRing, ModeSwitch, TodoList, SessionList,
                SettingsPanel, Botanical, Icons
public/fonts/   bundled Fraunces + Karla variable fonts (OFL)
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

- Phase 2: app usage tracking on COSMIC via the `cosmic-toplevel-info` Wayland
  protocol, idle/lock handling, and a local analytics view.
- Phase 3: native libcosmic panel applet over D-Bus, one-click panel timer, and
  idle-aware tracking.

## Notes

- The completion sound is played through `pw-play` or `paplay` with a bundled
  WAV, so no audio dev libraries are needed.
- Fraunces and Karla are bundled under the SIL Open Font License; license files
  are in `public/fonts/`.
- All artwork is inline SVG or CSS; no images are fetched at runtime.
- Autostart is opt-in from Settings. In `tauri dev` it registers the debug
  binary, which is not useful after development.
- The tracker will be COSMIC-specific; Wayland prevents generic global window
  inspection.
