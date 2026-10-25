export type TimerStatus = "idle" | "running" | "paused" | "finished";
export type TimerMode = "countdown" | "stopwatch";
export type ThemePreference = "system" | "light" | "dark";

export interface TimerSnapshot {
  status: TimerStatus;
  mode: TimerMode;
  plannedMs: number;
  remainingMs: number;
  elapsedMs: number;
  progress: number;
  endsAtMs: number | null;
  sessionStartedAtMs: number | null;
  stopwatchBaseMs: number;
  stopwatchStartedAtMs: number | null;
}

export interface Settings {
  soundEnabled: boolean;
  notificationsEnabled: boolean;
  defaultMinutes: number;
  theme: ThemePreference;
}

export interface SettingsView {
  settings: Settings;
  autostartEnabled: boolean;
}

export interface SessionRecord {
  id: number;
  plannedSeconds: number;
  actualSeconds: number;
  startedAtMs: number;
  endedAtMs: number;
  status: string;
}

export interface Todo {
  id: number;
  text: string;
  done: boolean;
  createdAtMs: number;
  updatedAtMs: number;
}
