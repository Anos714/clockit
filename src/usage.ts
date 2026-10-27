const KNOWN_APPS: Record<string, string> = {
  firefox: "Firefox",
  "org.mozilla.firefox": "Firefox",
  "google-chrome": "Google Chrome",
  chromium: "Chromium",
  "org.chromium.chromium": "Chromium",
  "code": "VS Code",
  "code-oss": "VS Code",
  "com.visualstudio.code": "VS Code",
  "org.gnome.nautilus": "Files",
  "org.gnome.console": "Console",
  kgx: "Console",
  "cosmic-term": "Terminal",
  "com.system76.cosmicterm": "Terminal",
  "org.wezfurlong.wezterm": "WezTerm",
  alacritty: "Alacritty",
  kitty: "Kitty",
  spotify: "Spotify",
  "com.spotify.client": "Spotify",
  discord: "Discord",
  "com.discordapp.discord": "Discord",
  slack: "Slack",
  "com.slack.slack": "Slack",
  obsidian: "Obsidian",
  "md.obsidian.obsidian": "Obsidian",
  "org.telegram.desktop": "Telegram",
  "com.system76.cosmicstore": "COSMIC Store",
  "com.system76.cosmicsettings": "Settings",
  "org.gnome.settings": "Settings",
  "cosmic-files": "Files",
  "cosmic-edit": "Editor",
  "org.gnome.texteditor": "Text Editor",
};

export function prettyAppId(appId: string): string {
  const cleaned = appId.trim();
  if (!cleaned) return "Unknown app";

  const known = KNOWN_APPS[cleaned.toLowerCase()];
  if (known) return known;

  const segments = cleaned.split(".").filter(Boolean);
  const tail = segments.length > 1 ? segments[segments.length - 1] : cleaned;

  return tail
    .replace(/[-_]+/g, " ")
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .split(" ")
    .filter(Boolean)
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");
}
