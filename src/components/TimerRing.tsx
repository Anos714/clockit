import {
  formatMinutes,
  splitCountdown,
  splitStopwatch,
} from "../format";
import type { TimerMode, TimerStatus } from "../types";

const RADIUS = 142;
const CIRCUMFERENCE = 2 * Math.PI * RADIUS;
const TICKS = Array.from({ length: 60 }, (_, index) => index);

interface TimerRingProps {
  mode: TimerMode;
  displayMs: number;
  plannedMs: number;
  progress: number;
  status: TimerStatus;
}

const countdownCaptions: Record<TimerStatus, string> = {
  idle: "ready when you are",
  running: "in focus",
  paused: "paused",
  finished: "session complete",
};

export function TimerRing({
  mode,
  displayMs,
  plannedMs,
  progress,
  status,
}: TimerRingProps) {
  const stopwatch = mode === "stopwatch";
  const clock = stopwatch ? splitStopwatch(displayMs) : splitCountdown(displayMs);
  const text = clock.main;
  const caption = stopwatch
    ? status === "running"
      ? "counting up"
      : status === "paused"
        ? "paused"
        : "ready when you are"
    : status === "idle"
      ? "ready when you are"
      : countdownCaptions[status];

  return (
    <div className={`ring-wrap ${status} ${mode}`}>
      <svg
        className="ring"
        viewBox="0 0 320 320"
        role="img"
        aria-label={
          stopwatch
            ? `${text} elapsed`
            : `${text} remaining in ${formatMinutes(plannedMs)} session`
        }
      >
        <defs>
          <linearGradient id="ring-gradient" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0%" stopColor="var(--ring-a)" />
            <stop offset="58%" stopColor="var(--ring-b)" />
            <stop offset="100%" stopColor="var(--ring-c)" />
          </linearGradient>
        </defs>

        <g className="ring-ticks" aria-hidden="true">
          {TICKS.map((tick) => (
            <line
              key={tick}
              x1="160"
              y1={tick % 5 === 0 ? 5 : 9}
              x2="160"
              y2={tick % 5 === 0 ? 13 : 14}
              transform={`rotate(${tick * 6} 160 160)`}
            />
          ))}
        </g>

        <circle className="ring-track" cx="160" cy="160" r={RADIUS} />
        <circle className="ring-inner" cx="160" cy="160" r="113" />
        <circle
          className="ring-progress"
          cx="160"
          cy="160"
          r={RADIUS}
          strokeDasharray={CIRCUMFERENCE}
          strokeDashoffset={CIRCUMFERENCE * (1 - progress)}
          transform="rotate(-90 160 160)"
        />

        <g
          className="ring-tip-group"
          transform={`rotate(${Math.min(1, Math.max(0, progress)) * 360} 160 160)`}
          aria-hidden="true"
        >
          <circle className="ring-tip" cx="160" cy="18" r="3.2" />
          <path
            className="ring-tip-leaf"
            d="M164 18c3.6-5 9.6-5 13.2 0-3.6 5-9.6 5-13.2 0Z"
          />
        </g>
      </svg>

      <div className="ring-center">
        <span className={`time ${text.length > 5 ? "time-sm" : ""}`}>
          {text}
          <span className="time-fraction">.{clock.fraction}</span>
        </span>
        <span className={`ring-caption ${status}`}>
          <span className="status-dot" />
          {caption}
        </span>
        <span className="ring-footnote">
          {stopwatch ? "elapsed" : `${formatMinutes(plannedMs)} focus`}
        </span>
      </div>
    </div>
  );
}
