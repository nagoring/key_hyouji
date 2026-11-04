//! 設定の読み書き（v0.1 は表示モードと表示位置のみ GUI から変更可能）。
//!
//! 保存先: `<app_config_dir>/settings.json`
//! - Windows: `%APPDATA%\com.nagoring.keyhyouji\settings.json`
//! - macOS: `~/Library/Application Support/com.nagoring.keyhyouji/settings.json`

use crate::{DisplayMode, ModifierStyle};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Preset {
    TopLeft,
    TopCenter,
    TopRight,
    MiddleLeft,
    MiddleCenter,
    MiddleRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl Preset {
    pub const ALL: [Preset; 9] = [
        Preset::TopLeft,
        Preset::TopCenter,
        Preset::TopRight,
        Preset::MiddleLeft,
        Preset::MiddleCenter,
        Preset::MiddleRight,
        Preset::BottomLeft,
        Preset::BottomCenter,
        Preset::BottomRight,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Preset::TopLeft => "top-left",
            Preset::TopCenter => "top-center",
            Preset::TopRight => "top-right",
            Preset::MiddleLeft => "middle-left",
            Preset::MiddleCenter => "middle-center",
            Preset::MiddleRight => "middle-right",
            Preset::BottomLeft => "bottom-left",
            Preset::BottomCenter => "bottom-center",
            Preset::BottomRight => "bottom-right",
        }
    }

    pub fn from_id(id: &str) -> Option<Preset> {
        Preset::ALL.into_iter().find(|p| p.id() == id)
    }

    pub fn label(self) -> &'static str {
        match self {
            Preset::TopLeft => "左上",
            Preset::TopCenter => "上中央",
            Preset::TopRight => "右上",
            Preset::MiddleLeft => "左中央",
            Preset::MiddleCenter => "中央",
            Preset::MiddleRight => "右中央",
            Preset::BottomLeft => "左下",
            Preset::BottomCenter => "下中央",
            Preset::BottomRight => "右下",
        }
    }

    pub fn vertical(self) -> &'static str {
        match self {
            Preset::TopLeft | Preset::TopCenter | Preset::TopRight => "top",
            Preset::MiddleLeft | Preset::MiddleCenter | Preset::MiddleRight => "middle",
            _ => "bottom",
        }
    }

    pub fn horizontal(self) -> &'static str {
        match self {
            Preset::TopLeft | Preset::MiddleLeft | Preset::BottomLeft => "left",
            Preset::TopRight | Preset::MiddleRight | Preset::BottomRight => "right",
            _ => "center",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Position {
    Preset { preset: Preset },
    /// ドラッグで配置した位置（ウィンドウ左上の物理座標）。
    /// `align` はウィンドウ内でキー表示を寄せる方向（配置前のプリセットを引き継ぐ）。
    Custom {
        x: i32,
        y: i32,
        #[serde(default = "default_align")]
        align: Preset,
    },
}

fn default_align() -> Preset {
    Preset::BottomCenter
}

impl Position {
    /// ウィンドウ内でキー表示を寄せる方向。
    pub fn align(&self) -> Preset {
        match *self {
            Position::Preset { preset } => preset,
            Position::Custom { align, .. } => align,
        }
    }
}

/// 表示するモニター（W-05）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MonitorTarget {
    /// メインモニター
    Primary,
    /// マウスカーソルのあるモニター（追従）
    Cursor,
    /// アクティブウィンドウのあるモニター（追従）
    Active,
    /// 名前で固定
    Fixed { name: String },
}

impl MonitorTarget {
    /// 状況に応じてモニターが変わるか
    pub fn follows(&self) -> bool {
        matches!(self, MonitorTarget::Cursor | MonitorTarget::Active)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Animation {
    Fade,
    Slide,
    None,
}

/// 外観（2.3）。プリセットは設定画面側で各値に展開し、`preset` は表示用のラベルとして持つ。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Theme {
    /// "dark" | "light" | "keycap" | "minimal" | "custom"
    pub preset: String,
    pub font_family: String,
    pub font_size: f64,
    pub font_weight: u32,
    pub text_color: String,
    pub bg_color: String,
    pub bg_opacity: f64,
    pub border_radius: f64,
    pub border_width: f64,
    pub border_color: String,
    pub shadow: bool,
    pub animation: Animation,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            preset: "dark".into(),
            font_family: "system-ui".into(),
            font_size: 32.0,
            font_weight: 600,
            text_color: "#ffffff".into(),
            bg_color: "#000000".into(),
            bg_opacity: 0.7,
            border_radius: 12.0,
            border_width: 0.0,
            border_color: "#ffffff".into(),
            shadow: true,
            animation: Animation::Fade,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub display_mode: DisplayMode,
    pub merge_typing_ms: u64,
    pub display_duration_ms: u64,
    pub fade_duration_ms: u64,
    pub max_items: u32,
    pub modifier_style: ModifierStyle,
    pub show_modifier_only: bool,
    pub position: Position,
    /// 画面端からの余白（論理ピクセル）
    pub offset_x: f64,
    pub offset_y: f64,
    pub monitor: MonitorTarget,
    pub theme: Theme,
    /// 除外アプリ（P-03）。Windows は実行ファイル名（例: `KeePass.exe`）、macOS はアプリ名
    pub excluded_apps: Vec<String>,
    /// 一時停止ホットキー（例: `Control+Alt+Shift+KeyK`）
    pub pause_hotkey: String,
}

pub const DEFAULT_PAUSE_HOTKEY: &str = "Control+Alt+Shift+KeyK";

impl Default for Config {
    fn default() -> Self {
        Self {
            display_mode: DisplayMode::All,
            merge_typing_ms: 1000,
            display_duration_ms: 2000,
            fade_duration_ms: 300,
            max_items: 3,
            modifier_style: ModifierStyle::Symbol,
            show_modifier_only: false,
            position: Position::Preset { preset: Preset::BottomCenter },
            offset_x: 40.0,
            offset_y: 80.0,
            monitor: MonitorTarget::Primary,
            theme: Theme::default(),
            excluded_apps: Vec::new(),
            pause_hotkey: DEFAULT_PAUSE_HOTKEY.into(),
        }
    }
}

fn is_hex_color(s: &str) -> bool {
    let hex = s.strip_prefix('#').unwrap_or("");
    matches!(hex.len(), 3 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit())
}

impl Config {
    pub fn path(dir: &Path) -> PathBuf {
        dir.join("settings.json")
    }

    pub fn load(dir: &Path) -> Config {
        match fs::read_to_string(Self::path(dir)) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
                log::warn!("settings.json is invalid, using defaults: {e}");
                Config::default()
            }),
            Err(_) => Config::default(),
        }
    }

    pub fn save(&self, dir: &Path) {
        let result = fs::create_dir_all(dir).and_then(|_| {
            let json = serde_json::to_string_pretty(self).expect("config serializes");
            fs::write(Self::path(dir), json)
        });
        if let Err(e) = result {
            log::error!("failed to save settings: {e}");
        }
    }

    /// 値を有効範囲に収める（設定画面や手編集の不正値対策）。
    pub fn sanitized(mut self) -> Config {
        let d = Config::default();
        self.merge_typing_ms = self.merge_typing_ms.clamp(200, 5000);
        self.display_duration_ms = self.display_duration_ms.clamp(500, 15000);
        self.fade_duration_ms = self.fade_duration_ms.clamp(0, 3000);
        self.max_items = self.max_items.clamp(1, 10);
        self.offset_x = clamp_f(self.offset_x, 0.0, 1000.0, d.offset_x);
        self.offset_y = clamp_f(self.offset_y, 0.0, 1000.0, d.offset_y);

        let t = &mut self.theme;
        let dt = Theme::default();
        t.font_size = clamp_f(t.font_size, 10.0, 160.0, dt.font_size);
        t.font_weight = (t.font_weight.clamp(100, 900) / 100) * 100;
        t.bg_opacity = clamp_f(t.bg_opacity, 0.0, 1.0, dt.bg_opacity);
        t.border_radius = clamp_f(t.border_radius, 0.0, 100.0, dt.border_radius);
        t.border_width = clamp_f(t.border_width, 0.0, 20.0, dt.border_width);
        for (v, def) in [
            (&mut t.text_color, &dt.text_color),
            (&mut t.bg_color, &dt.bg_color),
            (&mut t.border_color, &dt.border_color),
        ] {
            if !is_hex_color(v) {
                *v = def.clone();
            }
        }
        // フォント名は CSS に入るので安全な文字だけ許可
        let font_ok = !t.font_family.trim().is_empty()
            && t.font_family.len() <= 200
            && t.font_family.chars().all(|c| c.is_alphanumeric() || " -_,'\"".contains(c));
        if !font_ok {
            t.font_family = dt.font_family;
        }

        let mut seen = std::collections::HashSet::new();
        self.excluded_apps = self
            .excluded_apps
            .into_iter()
            .map(|a| a.trim().to_owned())
            .filter(|a| !a.is_empty() && seen.insert(a.to_lowercase()))
            .collect();
        if self.pause_hotkey.trim().is_empty() {
            self.pause_hotkey = d.pause_hotkey;
        }
        self
    }

    /// フォアグラウンドのアプリが除外対象か。
    /// Windows は `.exe` の有無を区別せず、大文字小文字を無視して比較する。
    pub fn is_excluded(&self, app_name: &str) -> bool {
        fn norm(s: &str) -> String {
            let s = s.trim().to_lowercase();
            s.strip_suffix(".exe").map(str::to_owned).unwrap_or(s)
        }
        let name = norm(app_name);
        !name.is_empty() && self.excluded_apps.iter().any(|a| norm(a) == name)
    }

    pub fn aggregator_config(&self) -> crate::AggregatorConfig {
        crate::AggregatorConfig {
            display_mode: self.display_mode,
            merge_typing_ms: self.merge_typing_ms,
            repeat_window_ms: self.display_duration_ms,
            show_modifier_only: self.show_modifier_only,
            modifier_style: self.modifier_style,
            ..Default::default()
        }
    }
}

fn clamp_f(v: f64, min: f64, max: f64, default: f64) -> f64 {
    if v.is_finite() {
        v.clamp(min, max)
    } else {
        default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_clamps_and_rejects_bad_values() {
        let mut c = Config {
            max_items: 0,
            display_duration_ms: 10,
            excluded_apps: vec![" KeePass.exe ".into(), "keepass.EXE".into(), "".into()],
            pause_hotkey: " ".into(),
            ..Default::default()
        };
        c.theme.font_size = f64::NAN;
        c.theme.bg_opacity = 3.0;
        c.theme.text_color = "red; background: url(x)".into();
        c.theme.font_family = "x}</style>".into();
        c.theme.font_weight = 650;
        let c = c.sanitized();
        assert_eq!(c.max_items, 1);
        assert_eq!(c.display_duration_ms, 500);
        assert_eq!(c.excluded_apps, vec!["KeePass.exe".to_string()]);
        assert_eq!(c.pause_hotkey, DEFAULT_PAUSE_HOTKEY);
        assert_eq!(c.theme.font_size, 32.0);
        assert_eq!(c.theme.bg_opacity, 1.0);
        assert_eq!(c.theme.text_color, "#ffffff");
        assert_eq!(c.theme.font_family, "system-ui");
        assert_eq!(c.theme.font_weight, 600);
        // 正しい値はそのまま
        let mut ok = Config::default();
        ok.theme.font_family = "'Hiragino Sans', Meiryo, sans-serif".into();
        ok.theme.bg_color = "#12345678".into();
        assert_eq!(ok.clone().sanitized(), ok);
    }

    #[test]
    fn excluded_app_matching() {
        let c = Config { excluded_apps: vec!["KeePass.exe".into(), "1Password".into()], ..Default::default() };
        assert!(c.is_excluded("keepass.exe"));
        assert!(c.is_excluded("KeePass"));
        assert!(c.is_excluded("1password"));
        assert!(!c.is_excluded("notepad.exe"));
        assert!(!c.is_excluded(""));
    }

    #[test]
    fn monitor_target_serde() {
        let c: Config = serde_json::from_str(r#"{"monitor":{"type":"fixed","name":"DELL U2720Q"}}"#).unwrap();
        assert_eq!(c.monitor, MonitorTarget::Fixed { name: "DELL U2720Q".into() });
        assert_eq!(serde_json::to_string(&MonitorTarget::Cursor).unwrap(), r#"{"type":"cursor"}"#);
    }

    #[test]
    fn roundtrip_and_partial() {
        let c: Config = serde_json::from_str(r#"{"displayMode":"shortcutsOnly"}"#).unwrap();
        assert_eq!(c.display_mode, DisplayMode::ShortcutsOnly);
        assert_eq!(c.max_items, 3);
        let s = serde_json::to_string(&Config {
            position: Position::Custom { x: 10, y: -20, align: Preset::TopLeft },
            ..Default::default()
        })
        .unwrap();
        assert!(s.contains(r#""position":{"type":"custom","x":10,"y":-20,"align":"top-left"}"#));
        let back: Config = serde_json::from_str(&s).unwrap();
        assert_eq!(back.position.align(), Preset::TopLeft);
        // align 省略時は下中央
        let old: Config = serde_json::from_str(r#"{"position":{"type":"custom","x":1,"y":2}}"#).unwrap();
        assert_eq!(old.position, Position::Custom { x: 1, y: 2, align: Preset::BottomCenter });
    }
}
