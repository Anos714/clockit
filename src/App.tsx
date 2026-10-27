import { useCallback, useEffect, useState, type FormEvent } from "react";
import { AnimatePresence, MotionConfig, motion } from "motion/react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api } from "./api";
import { formatMinutes, splitCountdown, splitStopwatch } from "./format";
import { useTimer } from "./useTimer";
import { useTheme } from "./useTheme";
import type {
  SessionRecord,
  Settings,
  SettingsView,
  TimerMode,
  TimerStatus,
} from "./types";
import { TimerRing } from "./components/TimerRing";
import { ModeSwitch } from "./components/ModeSwitch";
import { SessionList } from "./components/SessionList";
import { SettingsPanel } from "./components/SettingsPanel";
import { Insights } from "./components/Insights";
import { TodoList } from "./components/TodoList";
import {
  CornerSprig,
  ForestBackdrop,
  FocusGrove,
  LeafGlyph,
  SproutMark,
} from "./components/Botanical";
import {
  ChartIcon,
  ClockIcon,
  LeafIcon,
  MaximizeIcon,
  MinimizeIcon,
  MoonIcon,
  PauseIcon,
  PipIcon,
  PlayIcon,
  ResetIcon,
  SlidersIcon,
  SunIcon,
} from "./components/Icons";

const PRESETS = [5, 15, 25, 45, 60];

function statusLabel(mode: TimerMode, status: TimerStatus) {
  if (mode === "stopwatch") {
    if (status === "running") return "Counting";
    if (status === "paused") return "Paused";
    return "Ready";
  }
  if (status === "running") return "In focus";
  if (status === "paused") return "Paused";
  if (status === "finished") return "Complete";
  return "Ready";
}

export default function App() {
  const {
    snapshot,
    mode,
    displayMs,
    progress,
    error: timerError,
    actions,
  } = useTimer();
  const { resolved, setTheme, toggleTheme } = useTheme();
  const [sessions, setSessions] = useState<SessionRecord[]>([]);
  const [settingsView, setSettingsView] = useState<SettingsView | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [showInsights, setShowInsights] = useState(false);
  const [focusMode, setFocusMode] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [customMinutes, setCustomMinutes] = useState("30");

  const status: TimerStatus = snapshot?.status ?? "idle";
  const plannedMs = snapshot?.plannedMs ?? 0;
  const locked =
    mode === "countdown" && (status === "running" || status === "paused");
  const error = timerError ?? actionError;
  const label = statusLabel(mode, status);

  const refreshSessions = useCallback(async () => {
    try {
      setSessions(await api.sessions(8));
    } catch (reason) {
      setActionError(String(reason));
    }
  }, []);

  useEffect(() => {
    void refreshSessions();
  }, [refreshSessions, snapshot?.status]);

  useEffect(() => {
    api
      .getSettings()
      .then((view) => {
        setSettingsView(view);
        setTheme(view.settings.theme, false);
      })
      .catch((reason) => setActionError(String(reason)));
  }, [setTheme]);

  useEffect(() => {
    getCurrentWindow()
      .isFullscreen()
      .then(setFocusMode)
      .catch(() => undefined);
  }, []);

  const applyFocus = useCallback(async (on: boolean) => {
    setFocusMode(on);
    try {
      await getCurrentWindow().setFullscreen(on);
    } catch (reason) {
      setActionError(String(reason));
    }
  }, []);

  const updateSettings = useCallback(
    async (patch: Partial<Settings>) => {
      if (!settingsView) return;
      const previous = settingsView;
      const optimistic: SettingsView = {
        ...settingsView,
        settings: { ...settingsView.settings, ...patch },
      };
      setSettingsView(optimistic);
      if (patch.theme) setTheme(patch.theme);
      try {
        const next = await api.updateSettings(optimistic.settings);
        setSettingsView(next);
        setActionError(null);
      } catch (reason) {
        setSettingsView(previous);
        setTheme(previous.settings.theme, false);
        setActionError(String(reason));
      }
    },
    [settingsView, setTheme],
  );

  const toggleAutostart = useCallback(async () => {
    if (!settingsView) return;
    try {
      const next = await api.setAutostart(!settingsView.autostartEnabled);
      setSettingsView(next);
      setActionError(null);
    } catch (reason) {
      setActionError(String(reason));
    }
  }, [settingsView]);

  const clearUsage = useCallback(async () => {
    try {
      await api.clearUsageHistory();
      setActionError(null);
    } catch (reason) {
      setActionError(String(reason));
      throw reason;
    }
  }, []);

  const openSettings = useCallback(() => {
    setShowInsights(false);
    setShowSettings(true);
  }, []);

  const openInsights = useCallback(() => {
    setShowSettings(false);
    setShowInsights(true);
  }, []);

  const switchTheme = useCallback(() => {
    if (settingsView) {
      void updateSettings({
        theme: resolved === "dark" ? "light" : "dark",
      });
    } else {
      toggleTheme();
    }
  }, [resolved, settingsView, toggleTheme, updateSettings]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.metaKey || event.ctrlKey || event.altKey) return;

      if (showSettings) {
        if (event.key === "Escape") {
          event.preventDefault();
          setShowSettings(false);
        }
        return;
      }

      if (showInsights) {
        if (event.key === "Escape") {
          event.preventDefault();
          setShowInsights(false);
        }
        return;
      }

      const target = event.target as HTMLElement | null;
      const tag = target?.tagName ?? "";
      if (
        ["INPUT", "TEXTAREA", "SELECT", "BUTTON"].includes(tag) ||
        target?.isContentEditable
      ) {
        return;
      }

      if (event.code === "Space") {
        event.preventDefault();
        void actions.toggle();
      } else if (event.key === "r" || event.key === "R") {
        event.preventDefault();
        void actions.reset();
      } else if (event.key === "f" || event.key === "F") {
        event.preventDefault();
        void applyFocus(!focusMode);
      } else if (event.key === "i" || event.key === "I") {
        event.preventDefault();
        if (!focusMode) openInsights();
      } else if (event.key === "Escape" && focusMode) {
        void applyFocus(false);
      }
    };

    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [actions, applyFocus, focusMode, openInsights, showInsights, showSettings]);

  const primary = () => {
    if (status === "running") {
      void actions.pause();
    } else if (status === "paused") {
      void actions.resume();
    } else {
      void actions.start();
    }
  };

  const switchMode = (next: TimerMode) => {
    if (next === mode) return;
    void actions.setMode(next);
  };

  const choosePreset = (minutes: number) => {
    if (locked) return;
    void actions.setDuration(minutes * 60);
  };

  const submitCustom = (event: FormEvent) => {
    event.preventDefault();
    const parsed = Number(customMinutes);
    if (!Number.isFinite(parsed)) return;
    const minutes = Math.min(1440, Math.max(1, Math.round(parsed)));
    setCustomMinutes(String(minutes));
    void actions.setDuration(minutes * 60);
  };

  if (!snapshot) {
    return (
      <MotionConfig reducedMotion="user">
        <div className="loading-screen">
          <div className="loading-mark">
            <SproutMark size={64} />
          </div>
          <p>Preparing your quiet place…</p>
          {error ? <p className="loading-error">{error}</p> : null}
        </div>
      </MotionConfig>
    );
  }

  if (focusMode) {
    return (
      <MotionConfig reducedMotion="user">
        <FocusView
          mode={mode}
          displayMs={displayMs}
          plannedMs={plannedMs}
          progress={progress}
          status={status}
          primary={primary}
          onReset={() => void actions.reset()}
          onExit={() => void applyFocus(false)}
        />
      </MotionConfig>
    );
  }

  return (
    <MotionConfig reducedMotion="user">
      <div className="app">
        <ForestBackdrop />

        <header className="topbar">
          <div className="brand">
            <SproutMark size={42} />
            <div className="brand-copy">
              <span className="brand-name">Tempo</span>
              <span className="brand-tag">a quiet focus timer</span>
            </div>
          </div>

          <div className="top-actions">
            <button
              className="icon-btn"
              onClick={switchTheme}
              title={`Switch to ${resolved === "dark" ? "light" : "dark"} theme`}
              aria-label={`Switch to ${resolved === "dark" ? "light" : "dark"} theme`}
            >
              <AnimatePresence mode="wait" initial={false}>
                <motion.span
                  key={resolved}
                  className="icon-swap"
                  initial={{ opacity: 0, rotate: -35, scale: 0.7 }}
                  animate={{ opacity: 1, rotate: 0, scale: 1 }}
                  exit={{ opacity: 0, rotate: 35, scale: 0.7 }}
                  transition={{ duration: 0.18 }}
                >
                  {resolved === "dark" ? (
                    <SunIcon size={19} />
                  ) : (
                    <MoonIcon size={19} />
                  )}
                </motion.span>
              </AnimatePresence>
            </button>
            <button
              className="icon-btn"
              onClick={openInsights}
              title="Insights (I)"
              aria-label="Open usage insights"
            >
              <ChartIcon />
            </button>
            <button
              className="icon-btn"
              onClick={() => void api.openMini()}
              title="Mini window"
              aria-label="Open mini window"
            >
              <PipIcon />
            </button>
            <button
              className="icon-btn"
              onClick={() => void applyFocus(true)}
              title="Focus mode (F)"
              aria-label="Enter focus mode"
            >
              <MaximizeIcon />
            </button>
            <button
              className="icon-btn"
              onClick={openSettings}
              title="Settings"
              aria-label="Open settings"
              disabled={!settingsView}
            >
              <SlidersIcon />
            </button>
          </div>
        </header>

        {error ? (
          <motion.div
            className="error-banner"
            role="alert"
            initial={{ opacity: 0, y: -6 }}
            animate={{ opacity: 1, y: 0 }}
          >
            {error}
          </motion.div>
        ) : null}

        <main className="content">
          <section className="card timer-card">
            <CornerSprig className="timer-sprig" />

            <div className="timer-head">
              <ModeSwitch mode={mode} onChange={switchMode} />
              <span className={`status-pill ${status}`}>
                <span className="status-dot" />
                {label}
              </span>
            </div>

            <TimerRing
              mode={mode}
              displayMs={displayMs}
              plannedMs={plannedMs}
              progress={progress}
              status={status}
            />

            <div className="duration-slot">
              <AnimatePresence mode="wait" initial={false}>
                {mode === "countdown" ? (
                  <motion.div
                    key="countdown"
                    className="duration-block"
                    initial={{ opacity: 0, y: 6 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, y: -6 }}
                    transition={{ duration: 0.2 }}
                  >
                    <span className="field-label">Session length</span>
                    <div className="presets" aria-label="Timer presets">
                      {PRESETS.map((minutes) => {
                        const active = plannedMs === minutes * 60_000 && !locked;
                        return (
                          <motion.button
                            key={minutes}
                            className={`chip ${active ? "active" : ""}`}
                            disabled={locked}
                            onClick={() => choosePreset(minutes)}
                            whileHover={locked ? undefined : { y: -2 }}
                            whileTap={locked ? undefined : { scale: 0.94 }}
                            title={
                              locked
                                ? "Reset the timer before changing duration"
                                : `Set a ${minutes} minute timer`
                            }
                          >
                            {minutes}m
                          </motion.button>
                        );
                      })}
                    </div>
                    <form className="custom-form" onSubmit={submitCustom}>
                      <label htmlFor="custom-minutes">Custom</label>
                      <input
                        id="custom-minutes"
                        type="number"
                        min={1}
                        max={1440}
                        value={customMinutes}
                        disabled={locked}
                        onChange={(event) =>
                          setCustomMinutes(event.currentTarget.value)
                        }
                      />
                      <span>min</span>
                      <button
                        className="small-btn"
                        type="submit"
                        disabled={locked}
                      >
                        Set
                      </button>
                    </form>
                  </motion.div>
                ) : (
                  <motion.div
                    key="stopwatch"
                    className="stopwatch-note"
                    initial={{ opacity: 0, y: 6 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, y: -6 }}
                    transition={{ duration: 0.2 }}
                  >
                    <LeafGlyph size={18} />
                    <p>
                      <strong>Count upward.</strong> Pause whenever. Reset keeps
                      the run in your trail.
                    </p>
                  </motion.div>
                )}
              </AnimatePresence>
            </div>

            <div className="controls">
              <motion.button
                className="primary-btn"
                onClick={primary}
                whileHover={{ y: -2 }}
                whileTap={{ scale: 0.975 }}
                transition={{ type: "spring", stiffness: 420, damping: 26 }}
              >
                {status === "running" ? (
                  <>
                    <PauseIcon size={19} />
                    Pause
                  </>
                ) : status === "paused" ? (
                  <>
                    <PlayIcon size={19} />
                    Resume
                  </>
                ) : (
                  <>
                    <PlayIcon size={19} />
                    Start
                  </>
                )}
              </motion.button>
              <motion.button
                className="icon-btn lg"
                onClick={() => void actions.reset()}
                disabled={status === "idle"}
                aria-label="Reset timer"
                title="Reset (R)"
                whileHover={{ y: -2 }}
                whileTap={{ scale: 0.94 }}
              >
                <ResetIcon />
              </motion.button>
              <motion.button
                className="icon-btn lg"
                onClick={() => void applyFocus(true)}
                aria-label="Enter focus mode"
                title="Focus mode (F)"
                whileHover={{ y: -2 }}
                whileTap={{ scale: 0.94 }}
              >
                <MaximizeIcon />
              </motion.button>
            </div>

            <p className="hint">
              <kbd>Space</kbd> start / pause <span>·</span> <kbd>R</kbd> reset{" "}
              <span>·</span> <kbd>F</kbd> focus <span>·</span> <kbd>I</kbd>{" "}
              insights
            </p>
          </section>

          <aside className="side">
            <TodoList />
            <SessionList sessions={sessions} />

            <section className="card tip-card">
              <CornerSprig className="tip-sprig" />
              <span className="tip-eyebrow">
                <LeafIcon size={13} /> Local by design
              </span>
              <h2>Rooted on this machine.</h2>
              <p>
                Every session and task lives in a local SQLite file. No accounts,
                no network, no telemetry.
              </p>
              <div className="tip-meta">
                <span>
                  <ClockIcon size={15} /> timer + stopwatch
                </span>
                <span>
                  <PipIcon size={15} /> panel + mini window
                </span>
                <span>
                  <LeafIcon size={15} /> grows with your rhythm
                </span>
              </div>
            </section>
          </aside>
        </main>

        <AnimatePresence>
          {showSettings && settingsView ? (
            <SettingsPanel
              key="settings"
              view={settingsView}
              onClose={() => setShowSettings(false)}
              onUpdate={updateSettings}
              onAutostart={toggleAutostart}
              onClearUsage={clearUsage}
              error={actionError}
            />
          ) : null}
        </AnimatePresence>

        <AnimatePresence>
          {showInsights ? (
            <Insights
              key="insights"
              onClose={() => setShowInsights(false)}
            />
          ) : null}
        </AnimatePresence>
      </div>
    </MotionConfig>
  );
}

interface FocusViewProps {
  mode: TimerMode;
  displayMs: number;
  plannedMs: number;
  progress: number;
  status: TimerStatus;
  primary: () => void;
  onReset: () => void;
  onExit: () => void;
}

function FocusView({
  mode,
  displayMs,
  plannedMs,
  progress,
  status,
  primary,
  onReset,
  onExit,
}: FocusViewProps) {
  const stopwatch = mode === "stopwatch";
  const clock = stopwatch ? splitStopwatch(displayMs) : splitCountdown(displayMs);
  const text = clock.main;
  const label =
    status === "running"
      ? stopwatch
        ? "Counting"
        : "In focus"
      : status === "paused"
        ? "Paused"
        : status === "finished"
          ? "Complete"
          : "Ready";

  const note =
    status === "running"
      ? stopwatch
        ? "No finish line. Just presence."
        : "One thing at a time."
      : status === "paused"
        ? "The clearing can wait."
        : status === "finished"
          ? "A session well kept."
          : "Begin when you're ready.";

  return (
    <div className={`focus-shell ${mode}`}>
      <ForestBackdrop variant="focus" />
      <FocusGrove />
      <div className="focus-progress" style={{ width: `${progress * 100}%` }}>
        <span className="focus-progress-tip">
          <LeafGlyph size={13} />
        </span>
      </div>

      <div className="focus-top">
        <span className="focus-eyebrow">
          <LeafIcon size={14} />
          {stopwatch ? "Stopwatch" : "Focus mode"}
        </span>
        <button
          className="icon-btn lg"
          onClick={onExit}
          aria-label="Exit focus mode"
          title="Exit focus mode (Esc)"
        >
          <MinimizeIcon />
        </button>
      </div>

      <div className="focus-center">
        <span className={`focus-status ${status}`}>
          <span className="status-dot" />
          {label}
        </span>

        <div
          className={`focus-time ${text.length > 5 ? "focus-time-sm" : ""}`}
        >
          {text}
          <span className="focus-fraction">.{clock.fraction}</span>
        </div>

        <p className="focus-sub">
          {stopwatch ? "counting up" : `${formatMinutes(plannedMs)} block`}
          <span>·</span>
          <kbd>Esc</kbd> to leave
        </p>
        <p className="focus-note">{note}</p>

        <div className="focus-actions">
          <motion.button
            className="primary-btn xl"
            onClick={primary}
            whileHover={{ y: -2 }}
            whileTap={{ scale: 0.975 }}
            transition={{ type: "spring", stiffness: 420, damping: 26 }}
          >
            {status === "running" ? (
              <>
                <PauseIcon size={22} />
                Pause
              </>
            ) : status === "paused" ? (
              <>
                <PlayIcon size={22} />
                Resume
              </>
            ) : (
              <>
                <PlayIcon size={22} />
                Start
              </>
            )}
          </motion.button>
          <motion.button
            className="ghost-btn"
            onClick={onReset}
            whileHover={{ y: -2 }}
            whileTap={{ scale: 0.97 }}
          >
            <ResetIcon size={18} />
            Reset
          </motion.button>
        </div>
      </div>

      <div className="focus-bottom">
        <span>{stopwatch ? "Stopwatch" : "Timer"}</span>
        <span className="focus-brand">
          <LeafGlyph size={13} /> Tempo
        </span>
      </div>
    </div>
  );
}
