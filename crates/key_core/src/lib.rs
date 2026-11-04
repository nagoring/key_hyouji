//! キーストローク表示アプリのコアロジック。
//!
//! - [`event`] — OS 非依存のキーイベント型
//! - [`format`] — キー・修飾キーの表示文字列化
//! - [`aggregator`] — 文字の結合 / 連打カウント / 表示モード判定
//! - [`hook`] — OS ごとのグローバルキーフック（Windows / macOS）

pub mod aggregator;
pub mod autostart;
pub mod config;
pub mod event;
pub mod foreground;
pub mod format;
pub mod hook;
pub mod ime;
pub mod secure;

pub use aggregator::{Aggregator, AggregatorConfig, DisplayItem, DisplayKind, DisplayMode, Update};
pub use event::{
    InputEvent, Key, KeyEvent, Modifiers, MouseButton, MouseEvent, NamedKey, ScrollDirection, ScrollEvent,
};
pub use format::{ModifierStyle, Platform};
