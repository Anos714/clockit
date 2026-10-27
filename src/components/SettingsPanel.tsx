import { useEffect, useState, type FormEvent, type ReactNode } from "react";
import { motion } from "motion/react";
import type { Settings, SettingsView, ThemePreference } from "../types";
import {
  CloseIcon,
  LeafIcon,
  MonitorIcon,
  MoonIcon,
  ShieldIcon,
  SunIcon,
} from "./Icons";
import { SproutMark } from "./Botanical";

interface SettingsPanelProps {
  view: SettingsView;
  onClose: () => void;
  onUpdate: (patch: Partial<Settings>) => Promise<void>;
  onAutostart: () => Promise<void>;
  onClearUsage: () => Promise<void>;
  error?: string | null;
}

const THEMES: {
  id: ThemePreference;
  label: string;
  description: string;
  icon: typeof SunIcon;
}[] = [
  {
    id: "system",
    label: "System",
    description: "Follow the light",
    icon: MonitorIcon,
  },
  { id: "light", label: "Daylight", description: "Warm & airy", icon: SunIcon },
  { id: "dark", label: "Night wood", description: "Low & quiet", icon: MoonIcon },
];

export function SettingsPanel({
  view,
  onClose,
  onUpdate,
  onAutostart,
  onClearUsage,
  error,
}: SettingsPanelProps) {
  const [minutes, setMinutes] = useState(String(view.settings.defaultMinutes));
  const [idleMinutes, setIdleMinutes] = useState(
    String(Math.round(view.settings.idleThresholdSecs / 60)),
  );
  const [clearState, setClearState] = useState<"idle" | "confirm" | "cleared">(
    "idle",
  );
  const [clearing, setClearing] = useState(false);

  useEffect(() => {
    setMinutes(String(view.settings.defaultMinutes));
  }, [view.settings.defaultMinutes]);

  useEffect(() => {
    setIdleMinutes(String(Math.round(view.settings.idleThresholdSecs / 60)));
  }, [view.settings.idleThresholdSecs]);

  const saveMinutes = (event: FormEvent) => {
    event.preventDefault();
    const parsed = Number(minutes);
    if (!Number.isFinite(parsed)) return;
    const clamped = Math.min(1440, Math.max(1, Math.round(parsed)));
    setMinutes(String(clamped));
    void onUpdate({ defaultMinutes: clamped });
  };

  const saveIdle = (event: FormEvent) => {
    event.preventDefault();
    const parsed = Number(idleMinutes);
    if (!Number.isFinite(parsed)) return;
    const clamped = Math.min(30, Math.max(1, Math.round(parsed)));
    setIdleMinutes(String(clamped));
    void onUpdate({ idleThresholdSecs: clamped * 60 });
  };

  const clearUsage = async () => {
    if (clearing || clearState === "cleared") return;
    if (clearState === "idle") {
      setClearState("confirm");
      return;
    }
    setClearing(true);
    try {
      await onClearUsage();
      setClearState("cleared");
    } catch {
      setClearState("idle");
    } finally {
      setClearing(false);
    }
  };

  return (
    <motion.div
      className="settings-backdrop"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={{ duration: 0.22 }}
      onMouseDown={onClose}
    >
      <motion.aside
        className="settings-panel"
        role="dialog"
        aria-modal="true"
        aria-label="Settings"
        initial={{ x: 36, opacity: 0.7 }}
        animate={{ x: 0, opacity: 1 }}
        exit={{ x: 52, opacity: 0 }}
        transition={{ type: "spring", stiffness: 380, damping: 38 }}
        onMouseDown={(event) => event.stopPropagation()}
      >
        <header className="settings-head">
          <div className="settings-title">
            <SproutMark size={34} />
            <div>
              <span className="card-eyebrow">Preferences</span>
              <h2>Settings</h2>
            </div>
          </div>
          <button
            className="icon-btn"
            onClick={onClose}
            aria-label="Close settings"
          >
            <CloseIcon size={18} />
          </button>
        </header>

        <section className="settings-section">
          <h3>Appearance</h3>
          <div className="theme-grid" role="group" aria-label="Color theme">
            {THEMES.map(({ id, label, description, icon: Icon }) => {
              const active = view.settings.theme === id;
              return (
                <motion.button
                  key={id}
                  type="button"
                  className={`theme-option ${active ? "active" : ""}`}
                  aria-pressed={active}
                  onClick={() => void onUpdate({ theme: id })}
                  whileHover={{ y: -2 }}
                  whileTap={{ scale: 0.97 }}
                >
                  <span className="theme-icon">
                    <Icon size={17} />
                  </span>
                  <span className="theme-label">{label}</span>
                  <span className="theme-description">{description}</span>
                </motion.button>
              );
            })}
          </div>
          <p className="field-hint">
            {view.settings.theme === "system"
              ? "Tempo follows your desktop's light and dark preference."
              : "Your choice is remembered on this machine."}
          </p>
        </section>

        <section className="settings-section">
          <h3>Timer</h3>
          <form className="default-form" onSubmit={saveMinutes}>
            <label htmlFor="default-minutes">Default focus length</label>
            <div className="inline-field">
              <input
                id="default-minutes"
                type="number"
                min={1}
                max={1440}
                value={minutes}
                onChange={(event) => setMinutes(event.currentTarget.value)}
              />
              <span>min</span>
              <button className="small-btn" type="submit">
                Save
              </button>
            </div>
            <p className="field-hint">
              Applied the next time the timer is idle.
            </p>
          </form>
        </section>

        <section className="settings-section">
          <h3>Alerts</h3>
          <SettingRow
            label="Completion sound"
            description="A soft chime when a session ends."
          >
            <Switch
              checked={view.settings.soundEnabled}
              onChange={() =>
                void onUpdate({
                  soundEnabled: !view.settings.soundEnabled,
                })
              }
            />
          </SettingRow>
          <SettingRow
            label="Desktop notification"
            description="A quiet notice when time is up."
          >
            <Switch
              checked={view.settings.notificationsEnabled}
              onChange={() =>
                void onUpdate({
                  notificationsEnabled: !view.settings.notificationsEnabled,
                })
              }
            />
          </SettingRow>
        </section>

        <section className="settings-section">
          <h3>Privacy &amp; insight</h3>
          <SettingRow
            label="Usage insights"
            description="Record app names and time spent. Window titles are never stored."
          >
            <Switch
              checked={view.settings.usageTrackingEnabled}
              onChange={() =>
                void onUpdate({
                  usageTrackingEnabled: !view.settings.usageTrackingEnabled,
                })
              }
            />
          </SettingRow>
          <form className="default-form" onSubmit={saveIdle}>
            <label htmlFor="idle-minutes">Pause tracking after</label>
            <div className="inline-field">
              <input
                id="idle-minutes"
                type="number"
                min={1}
                max={30}
                value={idleMinutes}
                disabled={!view.settings.usageTrackingEnabled}
                onChange={(event) => setIdleMinutes(event.currentTarget.value)}
              />
              <span>min idle</span>
              <button
                className="small-btn"
                type="submit"
                disabled={!view.settings.usageTrackingEnabled}
              >
                Save
              </button>
            </div>
            <p className="field-hint">
              Idle and locked time never counts. Recording pauses automatically.
            </p>
          </form>
          <p className="privacy-note">
            <ShieldIcon size={14} />
            <span>
              Everything stays in Tempo&apos;s local database. No network, no
              accounts, no telemetry.
            </span>
          </p>
          <button
            type="button"
            className={`danger-btn ${clearState === "confirm" ? "confirm" : ""} ${
              clearState === "cleared" ? "cleared" : ""
            }`}
            onClick={() => void clearUsage()}
            disabled={clearing || clearState === "cleared"}
          >
            {clearState === "confirm"
              ? "Click again to erase usage history"
              : clearState === "cleared"
                ? "Usage history cleared"
                : "Clear usage history"}
          </button>
        </section>

        <section className="settings-section">
          <h3>System</h3>
          <SettingRow
            label="Start with COSMIC"
            description="Wake hidden in the panel at login."
          >
            <Switch
              checked={view.autostartEnabled}
              onChange={() => void onAutostart()}
            />
          </SettingRow>
          {view.autostartEnabled ? (
            <p className="field-hint">
              Autostart uses the binary currently running Tempo.
            </p>
          ) : null}
          {error ? <p className="settings-error">{error}</p> : null}
        </section>

        <footer className="settings-footer">
          <span>
            <LeafIcon size={14} /> Tempo 0.1.0
          </span>
          <span>Everything stays on this machine.</span>
        </footer>
      </motion.aside>
    </motion.div>
  );
}

function SettingRow({
  label,
  description,
  children,
}: {
  label: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <div className="setting-row">
      <div>
        <p>{label}</p>
        <span>{description}</span>
      </div>
      {children}
    </div>
  );
}

function Switch({
  checked,
  onChange,
}: {
  checked: boolean;
  onChange: () => void;
}) {
  return (
    <motion.button
      type="button"
      role="switch"
      aria-checked={checked}
      className={`switch ${checked ? "on" : ""}`}
      onClick={onChange}
      whileTap={{ scale: 0.94 }}
    >
      <motion.span
        className="switch-thumb"
        animate={{ x: checked ? 19 : 0 }}
        transition={{ type: "spring", stiffness: 520, damping: 32 }}
      />
    </motion.button>
  );
}
