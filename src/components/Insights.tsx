import { useEffect, useMemo, useState } from "react";
import { motion } from "motion/react";
import { api } from "../api";
import {
  formatHour,
  formatShortDate,
  formatUsage,
  formatWeekday,
  formatWhen,
} from "../format";
import { prettyAppId } from "../usage";
import type { UsageSnapshot, UsageStatus } from "../types";
import { CornerSprig, LeafGlyph } from "./Botanical";
import {
  ChartIcon,
  CloseIcon,
  LeafIcon,
  ShieldIcon,
} from "./Icons";

interface InsightsProps {
  onClose: () => void;
}

function usageState(status: UsageStatus | null) {
  if (!status) return { tone: "quiet", label: "Reading…" };
  if (!status.tracking) return { tone: "off", label: "Tracking off" };
  if (!status.available) {
    return {
      tone: "off",
      label: status.reason ?? "Waiting for COSMIC window info",
    };
  }
  if (status.paused) {
    if (status.pauseReason === "locked") {
      return { tone: "quiet", label: "Paused · screen locked" };
    }
    if (status.pauseReason === "idle") {
      return { tone: "quiet", label: "Paused · away" };
    }
    return { tone: "quiet", label: "Paused" };
  }
  if (status.activeApp) {
    return { tone: "live", label: prettyAppId(status.activeApp) };
  }
  return { tone: "quiet", label: "No active window" };
}

export function Insights({ onClose }: InsightsProps) {
  const [snapshot, setSnapshot] = useState<UsageSnapshot | null>(null);
  const [status, setStatus] = useState<UsageStatus | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      try {
        const [nextStatus, nextSnapshot] = await Promise.all([
          api.usageStatus(),
          api.usageSnapshot(7),
        ]);
        if (cancelled) return;
        setStatus(nextStatus);
        setSnapshot(nextSnapshot);
        setError(null);
      } catch (reason) {
        if (!cancelled) setError(String(reason));
      }
    };

    void load();
    const timer = window.setInterval(() => void load(), 10_000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, []);

  const weekly = useMemo(() => {
    if (!snapshot) return [];
    const max = Math.max(1, ...snapshot.daily.map((day) => day.totalMs));
    const today = snapshot.daily[snapshot.daily.length - 1]?.day;
    return snapshot.daily.map((day) => ({
      ...day,
      today: day.day === today,
      percent: Math.round((day.totalMs / max) * 100),
    }));
  }, [snapshot]);

  const apps = useMemo(() => {
    if (!snapshot) return [];
    const max = Math.max(1, ...snapshot.apps.map((app) => app.totalMs));
    return snapshot.apps.map((app) => ({
      ...app,
      percent: Math.round((app.totalMs / max) * 100),
    }));
  }, [snapshot]);

  const hourly = useMemo(() => {
    if (!snapshot) return [];
    const max = Math.max(1, ...snapshot.hourly.map((hour) => hour.totalMs));
    const currentHour = new Date().getHours();
    return snapshot.hourly.map((hour) => ({
      ...hour,
      now: hour.hour === currentHour,
      percent: Math.round((hour.totalMs / max) * 100),
    }));
  }, [snapshot]);

  const state = usageState(status);
  const average = snapshot
    ? Math.round(snapshot.totalMs / Math.max(1, snapshot.days))
    : 0;
  const top = snapshot?.apps[0];
  const empty = snapshot !== null && snapshot.totalMs === 0;

  return (
    <motion.div
      className="insights-backdrop"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={{ duration: 0.22 }}
      onMouseDown={onClose}
    >
      <motion.section
        className="insights-panel"
        role="dialog"
        aria-modal="true"
        aria-label="Usage insights"
        initial={{ y: 22, opacity: 0.6, scale: 0.985 }}
        animate={{ y: 0, opacity: 1, scale: 1 }}
        exit={{ y: 26, opacity: 0, scale: 0.99 }}
        transition={{ type: "spring", stiffness: 340, damping: 34 }}
        onMouseDown={(event) => event.stopPropagation()}
      >
        <CornerSprig className="insights-sprig" />

        <header className="insights-head">
          <div className="insights-title">
            <span className="insights-mark">
              <ChartIcon size={19} />
            </span>
            <div>
              <span className="card-eyebrow">Local insight</span>
              <h2>Where the hours go</h2>
            </div>
          </div>
          <div className="insights-head-actions">
            <span className={`usage-live ${state.tone}`}>
              <span className="status-dot" />
              {state.label}
            </span>
            <button
              className="icon-btn"
              onClick={onClose}
              aria-label="Close insights"
            >
              <CloseIcon size={18} />
            </button>
          </div>
        </header>

        <div className="insights-body">
          {error ? (
            <div className="insight-error" role="alert">
              {error}
            </div>
          ) : null}

          {!snapshot ? (
            <div className="insight-loading">
              <motion.span
                animate={{ rotate: 360 }}
                transition={{ duration: 2.6, repeat: Infinity, ease: "linear" }}
              >
                <LeafGlyph size={26} />
              </motion.span>
              <p>Reading your trail…</p>
            </div>
          ) : (
            <>
              <div className="insight-hero">
                <div>
                  <span className="insight-hero-label">Last 7 days</span>
                  <p className="insight-total">
                    {snapshot.totalMs > 0 ? formatUsage(snapshot.totalMs) : "0m"}
                  </p>
                  <span className="insight-hero-sub">
                    {snapshot.todayMs > 0
                      ? `${formatUsage(snapshot.todayMs)} today`
                      : "A quiet start today"}
                    {status?.activeSinceMs
                      ? ` · ${prettyAppId(status.activeApp ?? "")} in focus`
                      : ""}
                  </span>
                </div>
                <div className="insight-stats">
                  <div>
                    <span>Today</span>
                    <strong>{formatUsage(snapshot.todayMs)}</strong>
                  </div>
                  <div>
                    <span>Daily average</span>
                    <strong>{formatUsage(average)}</strong>
                  </div>
                  <div>
                    <span>Top app</span>
                    <strong>{top ? prettyAppId(top.appId) : "—"}</strong>
                    <small>{top ? formatUsage(top.totalMs) : "no data yet"}</small>
                  </div>
                </div>
              </div>

              {empty ? (
                <div className="insight-empty">
                  <LeafIcon size={17} />
                  <p>
                    The trail is empty. Use your desktop as usual — Clockit will
                    sketch the shape of your week here.
                  </p>
                </div>
              ) : null}

              <section className="insight-card">
                <header className="insight-card-head">
                  <h3>This week</h3>
                  <span>
                    {formatShortDate(
                      new Date(`${snapshot.daily[0]?.day}T12:00:00`).getTime(),
                    )}{" "}
                    — today
                  </span>
                </header>
                <div className="insight-week" role="img" aria-label="Daily usage for the last seven days">
                  {weekly.map((day, index) => (
                    <div className="insight-day" key={day.day}>
                      <span className="insight-day-value">
                        {day.totalMs > 0 ? formatUsage(day.totalMs) : ""}
                      </span>
                      <div
                        className={`insight-day-track ${day.today ? "today" : ""}`}
                        title={`${formatWeekday(day.day)} · ${formatUsage(day.totalMs)}`}
                      >
                        {day.totalMs > 0 ? (
                          <motion.span
                            className="insight-day-fill"
                            initial={{ scaleY: 0 }}
                            animate={{ scaleY: 1 }}
                            transition={{
                              type: "spring",
                              stiffness: 160,
                              damping: 24,
                              delay: 0.04 * index,
                            }}
                            style={{ height: `${day.percent}%` }}
                          />
                        ) : null}
                      </div>
                      <span className={`insight-day-label ${day.today ? "today" : ""}`}>
                        {formatWeekday(day.day)}
                      </span>
                    </div>
                  ))}
                </div>
              </section>

              <div className="insight-columns">
                <section className="insight-card">
                  <header className="insight-card-head">
                    <h3>Most used</h3>
                    <span>{snapshot.apps.length} apps</span>
                  </header>
                  {apps.length === 0 ? (
                    <p className="insight-muted">Nothing recorded yet.</p>
                  ) : (
                    <ol className="insight-apps">
                      {apps.map((app, index) => (
                        <li key={app.appId}>
                          <span className="insight-rank">{index + 1}</span>
                          <div className="insight-app-copy">
                            <div className="insight-app-line">
                              <p>{prettyAppId(app.appId)}</p>
                              <span>{formatUsage(app.totalMs)}</span>
                            </div>
                            <div className="insight-app-track">
                              <motion.span
                                initial={{ scaleX: 0 }}
                                animate={{ scaleX: 1 }}
                                transition={{
                                  duration: 0.5,
                                  delay: 0.05 * index,
                                  ease: [0.22, 1, 0.36, 1],
                                }}
                                style={{ width: `${app.percent}%` }}
                              />
                            </div>
                          </div>
                        </li>
                      ))}
                    </ol>
                  )}
                </section>

                <section className="insight-card">
                  <header className="insight-card-head">
                    <h3>Today by hour</h3>
                    <span>{formatUsage(snapshot.todayMs)}</span>
                  </header>
                  <div className="insight-hours" role="img" aria-label="Usage by hour today">
                    {hourly.map((hour) => (                      <div
                        className={`insight-hour ${hour.now ? "now" : ""}`}
                        key={hour.hour}
                        title={`${formatHour(hour.hour)} · ${formatUsage(hour.totalMs)}`}
                      >
                        {hour.totalMs > 0 ? (
                          <motion.span
                            initial={{ scaleY: 0 }}
                            animate={{ scaleY: 1 }}
                            transition={{
                              duration: 0.45,
                              delay: hour.hour * 0.012,
                              ease: [0.22, 1, 0.36, 1],
                            }}
                            style={{ height: `${hour.percent}%` }}
                          />
                        ) : null}
                      </div>
                    ))}
                  </div>
                  <div className="insight-hours-axis">
                    <span>00</span>
                    <span>06</span>
                    <span>12</span>
                    <span>18</span>
                    <span>23</span>
                  </div>
                </section>
              </div>

              <section className="insight-card">
                <header className="insight-card-head">
                  <h3>Recent activity</h3>
                  <span>{snapshot.recent.length} windows</span>
                </header>
                {snapshot.recent.length === 0 ? (
                  <p className="insight-muted">
                    Switches between apps will appear here as they happen.
                  </p>
                ) : (
                  <ul className="insight-recent">
                    {snapshot.recent.map((record) => (
                      <li key={record.id}>
                        <span className="insight-recent-dot" />
                        <span className="insight-recent-app">
                          {prettyAppId(record.appId)}
                        </span>
                        <span className="insight-recent-when">
                          {formatWhen(record.startedAtMs)}
                        </span>
                        <span className="insight-recent-time">
                          {formatUsage(record.endedAtMs - record.startedAtMs)}
                        </span>
                      </li>
                    ))}
                  </ul>
                )}
              </section>

              {!status?.available && status?.reason ? (
                <p className="insight-note">{status.reason}</p>
              ) : null}
            </>
          )}
        </div>

        <footer className="insights-footer">
          <span>
            <ShieldIcon size={13} /> App names only · no window titles
          </span>
          <span>
            {snapshot?.trackedSinceMs
              ? `Tracking since ${formatShortDate(snapshot.trackedSinceMs)} · 90-day history`
              : "Stored locally · 90-day history"}
          </span>
        </footer>
      </motion.section>
    </motion.div>
  );
}
