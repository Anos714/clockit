import { motion } from "motion/react";
import type { TimerMode } from "../types";
import { ClockIcon, StopwatchIcon } from "./Icons";

const MODES: {
  id: TimerMode;
  label: string;
  icon: typeof ClockIcon;
}[] = [
  { id: "countdown", label: "Timer", icon: ClockIcon },
  { id: "stopwatch", label: "Stopwatch", icon: StopwatchIcon },
];

export function ModeSwitch({
  mode,
  onChange,
}: {
  mode: TimerMode;
  onChange: (mode: TimerMode) => void;
}) {
  return (
    <div className="mode-switch" role="tablist" aria-label="Timer mode">
      {MODES.map(({ id, label, icon: Icon }) => {
        const active = mode === id;
        return (
          <motion.button
            key={id}
            type="button"
            role="tab"
            aria-selected={active}
            className={`mode-option ${active ? "active" : ""}`}
            onClick={() => onChange(id)}
            whileTap={{ scale: 0.97 }}
            transition={{ type: "spring", stiffness: 500, damping: 32 }}
          >
            {active ? (
              <motion.span
                className="mode-pill"
                layoutId="mode-pill"
                transition={{ type: "spring", stiffness: 420, damping: 34 }}
              />
            ) : null}
            <Icon size={15} />
            <span>{label}</span>
          </motion.button>
        );
      })}
    </div>
  );
}
