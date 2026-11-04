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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

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
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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
