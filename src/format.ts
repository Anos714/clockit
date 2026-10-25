export function formatClock(ms: number): string {
  const total = Math.ceil(Math.max(0, ms) / 1000);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;
  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
  }
  return `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}

export function splitStopwatch(ms: number): {
  main: string;
  fraction: string;
} {
  const safe = Math.max(0, ms);
  const wholeSeconds = Math.floor(safe / 1000);
  const hours = Math.floor(wholeSeconds / 3600);
  const minutes = Math.floor((wholeSeconds % 3600) / 60);
  const seconds = wholeSeconds % 60;
  const centis = Math.floor((safe % 1000) / 10);
  const main =
    hours > 0
      ? `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`
      : `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
  return { main, fraction: String(centis).padStart(2, "0") };
}

export function formatStopwatch(ms: number): string {
  const { main, fraction } = splitStopwatch(ms);
  return `${main}.${fraction}`;
}

export function formatMinutes(ms: number): string {
  const minutes = Math.round(ms / 60_000);
  return `${minutes} min`;
}

export function formatDuration(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  const rest = seconds % 60;
  if (minutes === 0) return `${rest}s`;
  if (rest === 0) return `${minutes}m`;
  return `${minutes}m ${rest}s`;
}

export function formatWhen(ms: number): string {
  const date = new Date(ms);
  const now = new Date();
  const sameDay =
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate();
  if (sameDay) {
    return date.toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
    });
  }
  return date.toLocaleDateString([], {
    month: "short",
    day: "numeric",
  });
}
