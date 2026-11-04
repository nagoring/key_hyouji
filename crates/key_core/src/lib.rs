//! キーストローク表示アプリのコアロジック。
//!
//! - [`event`] — OS 非依存のキーイベント型
//! - [`format`] — キー・修飾キーの表示文字列化
//! - [`aggregator`] — 文字の結合 / 連打カウント / 表示モード判定
//! - [`hook`] — OS ごとのグローバルキーフック（Windows / macOS）

pub mod aggregator;
pub mod config;
pub mod event;
pub mod format;
pub mod hook;

pub use aggregator::{Aggregator, AggregatorConfig, DisplayItem, DisplayKind, DisplayMode, Update};
pub use event::{Key, KeyEvent, Modifiers, NamedKey};
pub use format::{ModifierStyle, Platform};
