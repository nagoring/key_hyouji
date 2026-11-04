// Rust 側（key_core::config など）と対応する型

export type DisplayMode = "all" | "shortcutsOnly";
export type ModifierStyle = "symbol" | "text";
export type Animation = "fade" | "slide" | "none";
export type PresetId =
  | "top-left" | "top-center" | "top-right"
  | "middle-left" | "middle-center" | "middle-right"
  | "bottom-left" | "bottom-center" | "bottom-right";

export type Position =
  | { type: "preset"; preset: PresetId }
  | { type: "custom"; x: number; y: number; align: PresetId };

export type MonitorTarget =
  | { type: "primary" }
  | { type: "cursor" }
  | { type: "active" }
  | { type: "fixed"; name: string };

export type Theme = {
  preset: string;
  fontFamily: string;
  fontSize: number;
  fontWeight: number;
  textColor: string;
  bgColor: string;
  bgOpacity: number;
  borderRadius: number;
  borderWidth: number;
  borderColor: string;
  shadow: boolean;
  animation: Animation;
};

export type Config = {
  displayMode: DisplayMode;
  mergeTypingMs: number;
  displayDurationMs: number;
  fadeDurationMs: number;
  maxItems: number;
  modifierStyle: ModifierStyle;
  showModifierOnly: boolean;
  position: Position;
  offsetX: number;
  offsetY: number;
  monitor: MonitorTarget;
  theme: Theme;
  excludedApps: string[];
  pauseHotkey: string;
};

export type HookStatus =
  | { state: "starting" | "running" | "needPermission" }
  | { state: "error"; message: string };

export type MonitorInfo = { name: string; width: number; height: number; primary: boolean };
export type AppInfo = { version: string; platform: "windows" | "mac"; configPath: string };

/** テーマのプリセット。フォント・サイズ・アニメーションは利用者の設定を引き継ぐ。 */
export const THEME_PRESETS: Record<string, { label: string; theme: Partial<Theme> }> = {
  dark: {
    label: "ダーク",
    theme: { textColor: "#ffffff", bgColor: "#000000", bgOpacity: 0.7, borderRadius: 12, borderWidth: 0, borderColor: "#ffffff", shadow: true, fontWeight: 600 },
  },
  light: {
    label: "ライト",
    theme: { textColor: "#1f2328", bgColor: "#ffffff", bgOpacity: 0.92, borderRadius: 12, borderWidth: 1, borderColor: "#d0d7de", shadow: true, fontWeight: 600 },
  },
  keycap: {
    label: "キーキャップ",
    theme: { textColor: "#222222", bgColor: "#f4f4f4", bgOpacity: 1, borderRadius: 8, borderWidth: 1, borderColor: "#b5b5b5", shadow: true, fontWeight: 700 },
  },
  minimal: {
    label: "ミニマル",
    theme: { textColor: "#ffffff", bgColor: "#000000", bgOpacity: 0, borderRadius: 0, borderWidth: 0, borderColor: "#ffffff", shadow: false, fontWeight: 700 },
  },
};

/** "#rgb" / "#rrggbb" / "#rrggbbaa" と不透明度から rgba() を作る */
export function rgba(hex: string, opacity: number): string {
  let h = hex.replace("#", "");
  if (h.length === 3) h = h.split("").map((c) => c + c).join("");
  const r = parseInt(h.slice(0, 2), 16) || 0;
  const g = parseInt(h.slice(2, 4), 16) || 0;
  const b = parseInt(h.slice(4, 6), 16) || 0;
  const a = h.length === 8 ? (parseInt(h.slice(6, 8), 16) / 255) * opacity : opacity;
  return `rgba(${r}, ${g}, ${b}, ${Math.round(a * 1000) / 1000})`;
}

/** テーマを CSS 変数として要素に適用する（オーバーレイと設定画面のプレビューで共用） */
export function applyThemeVars(el: HTMLElement, t: Theme) {
  const s = el.style;
  s.setProperty("--text", t.textColor);
  s.setProperty("--bg", rgba(t.bgColor, t.bgOpacity));
  s.setProperty("--font-size", `${t.fontSize}px`);
  s.setProperty("--font-family", t.fontFamily);
  s.setProperty("--font-weight", String(t.fontWeight));
  s.setProperty("--radius", `${t.borderRadius}px`);
  s.setProperty("--border", t.borderWidth > 0 ? `${t.borderWidth}px solid ${t.borderColor}` : "none");
  el.dataset.preset = t.preset;
  el.dataset.shadow = String(t.shadow);
  el.dataset.animation = t.animation;
}

/** ホットキー文字列の表示（Rust の hotkey_label と同じ規則） */
export function hotkeyLabel(spec: string, mac: boolean): string {
  const map: Record<string, [string, string]> = {
    CONTROL: ["⌃", "Ctrl"], CTRL: ["⌃", "Ctrl"],
    ALT: ["⌥", "Alt"], OPTION: ["⌥", "Alt"],
    SHIFT: ["⇧", "Shift"],
    SUPER: ["⌘", "Win"], CMD: ["⌘", "Win"], COMMAND: ["⌘", "Win"], META: ["⌘", "Win"],
  };
  const parts = spec.split("+").map((p) => p.trim()).filter(Boolean).map((p) => {
    const m = map[p.toUpperCase()];
    if (m) return mac ? m[0] : m[1];
    return p.replace(/^(Key|Digit)/, "").toUpperCase();
  });
  return parts.join(mac ? "" : "+");
}
