import { useCallback, useEffect, useMemo, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api } from "./api";
import type { TimerMode, TimerSnapshot } from "./types";

export function useTimer() {
  const [snapshot, setSnapshot] = useState<TimerSnapshot | null>(null);
  const [now, setNow] = useState(() => Date.now());
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    let unlisten: UnlistenFn[] = [];

    const setup = async () => {
      try {
        const initial = await api.getSnapshot();
        if (!cancelled) setSnapshot(initial);

        const offUpdate = await listen<TimerSnapshot>(
          "timer://update",
          (event) => {
            setSnapshot(event.payload);
          },
        );
        const offFinished = await listen<TimerSnapshot>(
          "timer://finished",
          (event) => {
            setSnapshot(event.payload);
          },
        );

        if (cancelled) {
          offUpdate();
          offFinished();
          return;
        }
        unlisten = [offUpdate, offFinished];
      } catch (reason) {
        if (!cancelled) setError(String(reason));
      }
    };

    void setup();

    return () => {
      cancelled = true;
      unlisten.forEach((off) => off());
    };
  }, []);

  const stopwatchTicking =
    snapshot?.mode === "stopwatch" && snapshot.status === "running";

  useEffect(() => {
    const tick = window.setInterval(
      () => setNow(Date.now()),
      stopwatchTicking ? 50 : 200,
    );
    return () => window.clearInterval(tick);
  }, [stopwatchTicking]);

  const run = useCallback(
    async (action: () => Promise<TimerSnapshot>) => {
      try {
        const next = await action();
        setSnapshot(next);
        setError(null);
      } catch (reason) {
        setError(String(reason));
      }
    },
    [],
  );

  const actions = useMemo(
    () => ({
      start: (seconds?: number) => run(() => api.start(seconds)),
      pause: () => run(() => api.pause()),
      resume: () => run(() => api.resume()),
      toggle: () => run(() => api.toggle()),
      reset: () => run(() => api.reset()),
      setDuration: (seconds: number) => run(() => api.setDuration(seconds)),
      setMode: (mode: TimerMode) => run(() => api.setMode(mode)),
    }),
    [run],
  );

  const mode = snapshot?.mode ?? "countdown";
  const plannedMs = snapshot?.plannedMs ?? 0;

  const remainingMs = snapshot
    ? snapshot.mode === "countdown"
      ? snapshot.status === "running" && snapshot.endsAtMs
        ? Math.max(0, snapshot.endsAtMs - now)
        : snapshot.remainingMs
      : 0
    : 0;

  const elapsedMs = snapshot
    ? snapshot.mode === "stopwatch"
      ? snapshot.stopwatchBaseMs +
        (snapshot.status === "running" && snapshot.stopwatchStartedAtMs
          ? Math.max(0, now - snapshot.stopwatchStartedAtMs)
          : 0)
      : Math.max(0, plannedMs - remainingMs)
    : 0;

  const progress =
    snapshot?.mode === "stopwatch"
      ? elapsedMs > 0
        ? (elapsedMs % 60_000) / 60_000
        : 0
      : plannedMs > 0
        ? Math.min(1, Math.max(0, elapsedMs / plannedMs))
        : 0;

  const displayMs = snapshot?.mode === "stopwatch" ? elapsedMs : remainingMs;

  return {
    snapshot,
    mode,
    remainingMs,
    elapsedMs,
    displayMs,
    progress,
    error,
    actions,
  };
}
