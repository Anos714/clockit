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
  usageTrackingEnabled: boolean;
  idleThresholdSecs: number;
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

export type UsagePauseReason = "idle" | "locked" | "disabled";

export interface UsageStatus {
  available: boolean;
  reason: string | null;
  tracking: boolean;
  paused: boolean;
  pauseReason: UsagePauseReason | null;
  activeApp: string | null;
  activeSinceMs: number | null;
  idle: boolean;
  locked: boolean;
  idleSupported: boolean;
  lockSupported: boolean;
}

export interface UsageAppTotal {
  appId: string;
  totalMs: number;
}

export interface UsageDayTotal {
  day: string;
  totalMs: number;
}

export interface UsageHourTotal {
  hour: number;
  totalMs: number;
}

export interface UsageRecord {
  id: number;
  appId: string;
  startedAtMs: number;
  endedAtMs: number;
}

export interface UsageSnapshot {
  days: number;
  totalMs: number;
  todayMs: number;
  apps: UsageAppTotal[];
  daily: UsageDayTotal[];
  hourly: UsageHourTotal[];
  recent: UsageRecord[];
  trackedSinceMs: number | null;
}
