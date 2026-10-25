import { useCallback, useEffect, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api } from "./api";
import {
  applyTheme,
  readCachedTheme,
  resolveTheme,
  setSystemTheme,
  type ResolvedTheme,
} from "./theme";
import type { SettingsView, ThemePreference } from "./types";

export function useTheme() {
  const [preference, setPreference] = useState<ThemePreference>(readCachedTheme);
  const [resolved, setResolved] = useState<ResolvedTheme>(() =>
    applyTheme(readCachedTheme()),
  );

  const setTheme = useCallback((next: ThemePreference, animate = true) => {
    setPreference(next);
    setResolved(applyTheme(next, animate));
  }, []);

  useEffect(() => {
    if (preference !== "system") return;
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => setResolved(applyTheme("system"));
    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  }, [preference]);

  useEffect(() => {
    let disposed = false;
    let unlisten: UnlistenFn | null = null;

    const applySystem = (theme: string | null) => {
      if (disposed || (theme !== "light" && theme !== "dark")) return;
      setSystemTheme(theme);
      if (preference === "system") {
        setResolved(applyTheme("system", false));
      }
    };

    try {
      const win = getCurrentWindow();
      win.theme().then(applySystem).catch(() => undefined);
      win
        .onThemeChanged((event) => applySystem(event.payload))
        .then((off) => {
          if (disposed) off();
          else unlisten = off;
        })
        .catch(() => undefined);
    } catch {
      // browser previews have no Tauri window
    }

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [preference]);

  useEffect(() => {
    let cancelled = false;
    let unlisten: UnlistenFn | null = null;

    api
      .getSettings()
      .then((view) => {
        if (!cancelled) setTheme(view.settings.theme, false);
      })
      .catch(() => undefined);

    listen<SettingsView>("settings://changed", (event) => {
      if (!cancelled) setTheme(event.payload.settings.theme);
    })
      .then((off) => {
        if (cancelled) off();
        else unlisten = off;
      })
      .catch(() => undefined);

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [setTheme]);

  const toggleTheme = useCallback(() => {
    setTheme(resolved === "dark" ? "light" : "dark");
  }, [resolved, setTheme]);

  return {
    preference,
    resolved,
    systemPrefersDark: resolveTheme("system") === "dark",
    setTheme,
    toggleTheme,
  };
}
