import type { ThemePreference } from "./types";

export type ResolvedTheme = "light" | "dark";

const CACHE_KEY = "tempo.theme";
const RESOLVED_KEY = "tempo.theme.resolved";

let systemTheme: ResolvedTheme | null = null;

export function setSystemTheme(theme: ResolvedTheme) {
  systemTheme = theme;
}

export function readCachedTheme(): ThemePreference {
  try {
    const value = localStorage.getItem(CACHE_KEY);
    if (value === "light" || value === "dark" || value === "system") {
      return value;
    }
  } catch {
    // localStorage can be unavailable in hardened webviews
  }
  return "system";
}

export function resolveTheme(preference: ThemePreference): ResolvedTheme {
  if (preference === "system") {
    if (systemTheme) return systemTheme;
    return window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  }
  return preference;
}

export function applyTheme(
  preference: ThemePreference,
  animate = false,
): ResolvedTheme {
  const resolved = resolveTheme(preference);
  const root = document.documentElement;

  if (animate) {
    root.classList.add("theme-changing");
    window.setTimeout(() => root.classList.remove("theme-changing"), 480);
  }

  root.dataset.theme = resolved;
  root.dataset.themePreference = preference;
  root.style.colorScheme = resolved;

  try {
    localStorage.setItem(CACHE_KEY, preference);
    localStorage.setItem(RESOLVED_KEY, resolved);
  } catch {
    // cache is best-effort only
  }

  return resolved;
}
