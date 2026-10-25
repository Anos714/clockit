import { formatDuration, formatMinutes, formatWhen } from "../format";
import type { SessionRecord } from "../types";
import { CornerSprig } from "./Botanical";
import { LeafIcon, StopwatchIcon } from "./Icons";

function sessionLabel(status: string) {
  if (status === "stopwatch") return "Stopwatch";
  if (status === "completed") return "Focus session";
  return "Ended early";
}

export function SessionList({ sessions }: { sessions: SessionRecord[] }) {
  return (
    <section className="card sessions-card">
      <CornerSprig />
      <header className="card-head">
        <div>
          <span className="card-eyebrow">Trail</span>
          <h2>Recent sessions</h2>
        </div>
        <span className="count">{sessions.length}</span>
      </header>

      {sessions.length === 0 ? (
        <div className="empty-state compact">
          <LeafIcon size={18} />
          <p>Your focus trail will appear here after the first session.</p>
        </div>
      ) : (
        <ul className="session-list">
          {sessions.map((session) => {
            const completed = session.status === "completed";
            const stopwatch = session.status === "stopwatch";
            return (
              <li key={session.id}>
                <div className="session-main">
                  <span
                    className={`session-mark ${
                      stopwatch
                        ? "stopwatch"
                        : completed
                          ? "completed"
                          : "cancelled"
                    }`}
                  >
                    {stopwatch ? (
                      <StopwatchIcon size={13} />
                    ) : completed ? (
                      <LeafIcon size={13} />
                    ) : (
                      <span className="session-dash" />
                    )}
                  </span>
                  <div className="session-copy">
                    <p>{sessionLabel(session.status)}</p>
                    <span>
                      {formatWhen(session.endedAtMs)}
                      {stopwatch
                        ? " · counted up"
                        : ` · ${formatMinutes(session.plannedSeconds * 1000)} planned`}
                    </span>
                  </div>
                </div>
                <span
                  className={`session-duration ${stopwatch || !completed ? "muted" : ""}`}
                >
                  {formatDuration(session.actualSeconds)}
                </span>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
