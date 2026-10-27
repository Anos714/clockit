use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub sound_enabled: bool,
    pub notifications_enabled: bool,
    pub default_minutes: u64,
    pub theme: Theme,
    pub usage_tracking_enabled: bool,
    pub idle_threshold_secs: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sound_enabled: true,
            notifications_enabled: true,
            default_minutes: 25,
            theme: Theme::System,
            usage_tracking_enabled: true,
            idle_threshold_secs: 120,
        }
    }
}

impl Settings {
    pub fn default_ms(&self) -> u64 {
        self.default_minutes.clamp(1, 1440) * 60_000
    }

    pub fn sanitize(&mut self) {
        self.default_minutes = self.default_minutes.clamp(1, 1440);
        self.idle_threshold_secs = self.idle_threshold_secs.clamp(30, 1800);
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    pub settings: Settings,
    pub autostart_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn older_settings_load_with_system_theme() {
        let mut settings: Settings =
            serde_json::from_str(r#"{"soundEnabled":true,"notificationsEnabled":false,"defaultMinutes":40}"#)
                .expect("legacy settings parse");
        assert_eq!(settings.theme, Theme::System);
        assert_eq!(settings.default_minutes, 40);
        assert!(settings.usage_tracking_enabled);
        assert_eq!(settings.idle_threshold_secs, 120);

        settings.default_minutes = 0;
        settings.idle_threshold_secs = 5;
        settings.sanitize();
        assert_eq!(settings.default_minutes, 1);
        assert_eq!(settings.idle_threshold_secs, 30);
    }
}
