import { invoke } from "@tauri-apps/api/core";
import type {
  SessionRecord,
  Settings,
  SettingsView,
  TimerMode,
  TimerSnapshot,
  Todo,
  UsageSnapshot,
  UsageStatus,
} from "./types";

export const api = {
  getSnapshot: () => invoke<TimerSnapshot>("get_snapshot"),
  start: (seconds?: number) =>
    invoke<TimerSnapshot>("start_timer", { seconds: seconds ?? null }),
  pause: () => invoke<TimerSnapshot>("pause_timer"),
  resume: () => invoke<TimerSnapshot>("resume_timer"),
  toggle: () => invoke<TimerSnapshot>("toggle_timer"),
  reset: () => invoke<TimerSnapshot>("reset_timer"),
  setDuration: (seconds: number) =>
    invoke<TimerSnapshot>("set_duration", { seconds }),
  setMode: (mode: TimerMode) => invoke<TimerSnapshot>("set_mode", { mode }),
  sessions: (limit = 8) =>
    invoke<SessionRecord[]>("get_sessions", { limit }),
  todos: () => invoke<Todo[]>("get_todos"),
  addTodo: (text: string) => invoke<Todo>("add_todo", { text }),
  setTodoDone: (id: number, done: boolean) =>
    invoke<Todo>("set_todo_done", { id, done }),
  updateTodoText: (id: number, text: string) =>
    invoke<Todo>("update_todo_text", { id, text }),
  deleteTodo: (id: number) => invoke<void>("delete_todo", { id }),
  getSettings: () => invoke<SettingsView>("get_settings"),
  updateSettings: (settings: Settings) =>
    invoke<SettingsView>("update_settings", { settings }),
  setAutostart: (enabled: boolean) =>
    invoke<SettingsView>("set_autostart", { enabled }),
  usageStatus: () => invoke<UsageStatus>("get_usage_status"),
  usageSnapshot: (days = 7) =>
    invoke<UsageSnapshot>("get_usage_snapshot", { days }),
  clearUsageHistory: () => invoke<void>("clear_usage_history"),
  openMini: () => invoke<void>("open_mini"),
  showMain: () => invoke<void>("show_main"),
  hideMini: () => invoke<void>("hide_mini"),
};
