import { MotionConfig, motion } from "motion/react";
import { api } from "./api";
import { formatClock, splitStopwatch } from "./format";
import { useTimer } from "./useTimer";
import { useTheme } from "./useTheme";
import { LeafGlyph } from "./components/Botanical";
import {
  CloseIcon,
  MaximizeIcon,
  PauseIcon,
  PlayIcon,
  ResetIcon,
} from "./components/Icons";

export default function MiniApp() {
  const { snapshot, mode, displayMs, actions } = useTimer();
  useTheme();

  const status = snapshot?.status ?? "idle";
  const stopwatch = mode === "stopwatch";
  const clock = splitStopwatch(displayMs);
  const text = stopwatch ? clock.main : formatClock(displayMs);

  const primary = () => {
    if (status === "running") {
      void actions.pause();
    } else if (status === "paused") {
      void actions.resume();
    } else {
      void actions.start();
    }
  };

  const expand = async () => {
    await api.showMain();
    await api.hideMini();
  };

  return (
    <MotionConfig reducedMotion="user">
      <div className={`mini ${mode}`}>
        <span className="mini-vine" aria-hidden="true" />

        <div className="mini-head" data-tauri-drag-region>
          <span className={`mini-dot ${status}`} />
          <span className="mini-label">Tempo</span>
          <span className="mini-mode">{stopwatch ? "stopwatch" : "timer"}</span>
          <span className="mini-spacer" data-tauri-drag-region />
          <button
            className="mini-icon"
            onClick={() => void api.hideMini()}
            aria-label="Hide mini window"
            title="Hide"
          >
            <CloseIcon size={14} />
          </button>
        </div>

        <div className={`mini-time ${status}`}>
          {snapshot ? (
            <>
              {text}
              {stopwatch ? (
                <span className="mini-fraction">.{clock.fraction}</span>
              ) : null}
            </>
          ) : (
            "--:--"
          )}
        </div>

        <div className="mini-controls">
          <motion.button
            className="mini-primary"
            onClick={primary}
            whileHover={{ y: -1 }}
            whileTap={{ scale: 0.97 }}
          >
            {status === "running" ? (
              <PauseIcon size={15} />
            ) : (
              <PlayIcon size={15} />
            )}
            <span>
              {status === "running"
                ? "Pause"
                : status === "paused"
                  ? "Resume"
                  : "Start"}
            </span>
          </motion.button>
          <motion.button
            className="mini-icon wide"
            onClick={() => void actions.reset()}
            disabled={status === "idle"}
            aria-label="Reset timer"
            title="Reset"
            whileTap={{ scale: 0.94 }}
          >
            <ResetIcon size={16} />
          </motion.button>
          <motion.button
            className="mini-icon wide"
            onClick={() => void expand()}
            aria-label="Open Tempo"
            title="Open full window"
            whileTap={{ scale: 0.94 }}
          >
            <MaximizeIcon size={16} />
          </motion.button>
        </div>

        <span className="mini-spout" aria-hidden="true">
          <LeafGlyph size={12} />
        </span>
      </div>
    </MotionConfig>
  );
}
