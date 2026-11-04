//! キーイベントを「表示アイテム」に変換する。
//!
//! - F-03: 修飾なしの文字入力は一定時間内なら 1 つのアイテムに結合（例: `hello`）
//! - F-04: 同じ組み合わせの連打は回数を加算（例: `⌘ Z ×3`）
//! - F-09: 表示モード（すべて / ショートカットのみ）
//! - F-12: 修飾キー単体は既定で表示しない
//!
//! 何件まで表示するか・いつ消すかはフロントエンド側の責務。

use crate::event::{Key, KeyEvent, Modifiers, MouseButton, NamedKey, ScrollDirection};
use crate::format::{self, ModifierStyle, Platform};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DisplayMode {
    /// すべてのキーを表示
    All,
    /// 修飾キー付き（ショートカット）のみ
    ShortcutsOnly,
}

#[derive(Debug, Clone)]
pub struct AggregatorConfig {
    pub display_mode: DisplayMode,
    /// 文字入力を結合する最大間隔
    pub merge_typing_ms: u64,
    /// 同じ組み合わせを連打とみなす最大間隔
    pub repeat_window_ms: u64,
    /// 1 アイテムに結合する最大文字数
    pub max_text_len: usize,
    pub show_modifier_only: bool,
    pub platform: Platform,
    pub modifier_style: ModifierStyle,
    /// クリックの表示名（左 / 右 / 中）。言語に合わせてアプリ側で設定する
    pub click_labels: [String; 3],
    /// スクロールの表示名（例: "スクロール" → `スクロール ↓`）
    pub scroll_label: String,
    /// この間隔より短いスクロールは 1 回分にまとめる（トラックパッドの細かいイベント対策）
    pub scroll_min_interval_ms: u64,
}

impl Default for AggregatorConfig {
    fn default() -> Self {
        Self {
            display_mode: DisplayMode::All,
            merge_typing_ms: 1000,
            repeat_window_ms: 2000,
            max_text_len: 40,
            show_modifier_only: false,
            platform: Platform::current(),
            modifier_style: ModifierStyle::Symbol,
            click_labels: ["Click".into(), "Right Click".into(), "Middle Click".into()],
            scroll_label: "Scroll".into(),
            scroll_min_interval_ms: 80,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DisplayKind {
    /// 結合された文字入力
    Text,
    /// ショートカット / 特殊キー
    Combo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DisplayItem {
    pub id: u64,
    pub kind: DisplayKind,
    pub label: String,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", content = "item", rename_all = "lowercase")]
pub enum Update {
    /// 新しいアイテムを追加
    New(DisplayItem),
    /// 既存アイテム（同じ id）の内容を更新し、表示タイマーをリセット
    Update(DisplayItem),
}

struct Last {
    item: DisplayItem,
    at_ms: u64,
}

pub struct Aggregator {
    config: AggregatorConfig,
    next_id: u64,
    last: Option<Last>,
}

impl Aggregator {
    pub fn new(config: AggregatorConfig) -> Self {
        Self { config, next_id: 1, last: None }
    }

    pub fn config(&self) -> &AggregatorConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: AggregatorConfig) {
        self.config = config;
        self.last = None;
    }

    /// 直前のアイテムとの結合を打ち切る（一時停止時など）。
    pub fn reset(&mut self) {
        self.last = None;
    }

    /// キーイベントを処理し、表示更新があれば返す。`now_ms` は単調増加する時刻。
    pub fn push(&mut self, ev: &KeyEvent, now_ms: u64) -> Option<Update> {
        if let Key::Named(n) = &ev.key {
            if n.is_modifier() {
                if !self.config.show_modifier_only || self.config.display_mode == DisplayMode::ShortcutsOnly {
                    return None;
                }
                let label = format::key_label(&ev.key, self.config.platform);
                return Some(self.combo(label, now_ms));
            }
        }

        if let Some(text) = typing_text(ev) {
            if self.config.display_mode == DisplayMode::ShortcutsOnly {
                self.last = None;
                return None;
            }
            return Some(self.typing(&text, now_ms));
        }

        if self.config.display_mode == DisplayMode::ShortcutsOnly && !is_shortcut(ev) {
            self.last = None;
            return None;
        }
        let label = format::combo_label(&ev.key, ev.modifiers, self.config.platform, self.config.modifier_style);
        Some(self.combo(label, now_ms))
    }

    /// マウスクリックを処理する（F-10）。同じクリックの連続は回数を加算（ダブルクリックは ×2）。
    pub fn push_click(&mut self, button: MouseButton, modifiers: Modifiers, now_ms: u64) -> Option<Update> {
        if self.config.display_mode == DisplayMode::ShortcutsOnly && !modifiers.has_command() {
            self.last = None;
            return None;
        }
        let idx = match button {
            MouseButton::Left => 0,
            MouseButton::Right => 1,
            MouseButton::Middle => 2,
        };
        let key = Key::Unknown(self.config.click_labels[idx].clone());
        let label = format::combo_label(&key, modifiers, self.config.platform, self.config.modifier_style);
        Some(self.combo(label, now_ms))
    }

    /// ホイールの回転を処理する。同じ向きの連続は回数を加算し、
    /// `scroll_min_interval_ms` より短い間隔のイベントは数えない。
    pub fn push_scroll(&mut self, direction: ScrollDirection, modifiers: Modifiers, now_ms: u64) -> Option<Update> {
        if self.config.display_mode == DisplayMode::ShortcutsOnly && !modifiers.has_command() {
            self.last = None;
            return None;
        }
        let key = Key::Unknown(format!("{} {}", self.config.scroll_label, direction.arrow()));
        let label = format::combo_label(&key, modifiers, self.config.platform, self.config.modifier_style);
        if let Some(last) = &self.last {
            let same = last.item.kind == DisplayKind::Combo && last.item.label == label;
            if same && now_ms.saturating_sub(last.at_ms) < self.config.scroll_min_interval_ms {
                return None;
            }
        }
        Some(self.combo(label, now_ms))
    }

    fn typing(&mut self, text: &str, now_ms: u64) -> Update {
        if let Some(last) = &mut self.last {
            let within = now_ms.saturating_sub(last.at_ms) <= self.config.merge_typing_ms;
            let fits = last.item.label.chars().count() + text.chars().count() <= self.config.max_text_len;
            if last.item.kind == DisplayKind::Text && within && fits {
                last.item.label.push_str(text);
                last.at_ms = now_ms;
                return Update::Update(last.item.clone());
            }
        }
        Update::New(self.start(DisplayKind::Text, text.to_owned(), now_ms))
    }

    fn combo(&mut self, label: String, now_ms: u64) -> Update {
        if let Some(last) = &mut self.last {
            let within = now_ms.saturating_sub(last.at_ms) <= self.config.repeat_window_ms;
            if last.item.kind == DisplayKind::Combo && within && last.item.label == label {
                last.item.count += 1;
                last.at_ms = now_ms;
                return Update::Update(last.item.clone());
            }
        }
        Update::New(self.start(DisplayKind::Combo, label, now_ms))
    }

    fn start(&mut self, kind: DisplayKind, label: String, now_ms: u64) -> DisplayItem {
        let item = DisplayItem { id: self.next_id, kind, label, count: 1 };
        self.next_id += 1;
        self.last = Some(Last { item: item.clone(), at_ms: now_ms });
        item
    }
}

/// 修飾なし（Shift は可）の文字入力か（IME オン中に隠す対象）。
pub fn is_typing(ev: &KeyEvent) -> bool {
    typing_text(ev).is_some()
}

/// 修飾なし（Shift は可）で印字可能な文字を入力するキーなら、表示用の文字列を返す。
fn typing_text(ev: &KeyEvent) -> Option<String> {
    if ev.modifiers.has_command() {
        return None;
    }
    if matches!(ev.key, Key::Named(NamedKey::Space)) {
        return Some("␣".to_owned());
    }
    if let Key::Named(_) = ev.key {
        return None;
    }
    let text = ev.text.as_deref()?;
    if text.is_empty() || text.chars().any(char::is_control) {
        return None;
    }
    Some(text.replace(' ', "␣"))
}

/// ショートカットのみモードで表示するか。
fn is_shortcut(ev: &KeyEvent) -> bool {
    ev.modifiers.has_command() || (ev.modifiers.shift && matches!(ev.key, Key::Named(_)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Modifiers;

    fn cfg(platform: Platform) -> AggregatorConfig {
        AggregatorConfig { platform, ..Default::default() }
    }

    fn ch(c: &str) -> KeyEvent {
        KeyEvent::new(Key::Char(c.to_lowercase()), Some(c), Modifiers::NONE)
    }

    fn shifted(base: &str, text: &str) -> KeyEvent {
        KeyEvent::new(Key::Char(base.into()), Some(text), Modifiers { shift: true, ..Modifiers::NONE })
    }

    fn cmd(c: &str) -> KeyEvent {
        KeyEvent::new(Key::Char(c.into()), Some(c), Modifiers { meta: true, ..Modifiers::NONE })
    }

    fn named(n: NamedKey) -> KeyEvent {
        KeyEvent::new(Key::Named(n), None, Modifiers::NONE)
    }

    fn label(u: &Update) -> (&'static str, u64, &str, u32) {
        match u {
            Update::New(i) => ("new", i.id, i.label.as_str(), i.count),
            Update::Update(i) => ("update", i.id, i.label.as_str(), i.count),
        }
    }

    #[test]
    fn merges_typing_within_window() {
        let mut a = Aggregator::new(cfg(Platform::Windows));
        assert_eq!(label(&a.push(&shifted("h", "H"), 0).unwrap()), ("new", 1, "H", 1));
        assert_eq!(label(&a.push(&ch("i"), 100).unwrap()), ("update", 1, "Hi", 1));
        assert_eq!(label(&a.push(&named(NamedKey::Space), 200).unwrap()), ("update", 1, "Hi␣", 1));
        // 間隔が空いたら新しいアイテム
        assert_eq!(label(&a.push(&ch("x"), 1500).unwrap()), ("new", 2, "x", 1));
    }

    #[test]
    fn typing_respects_max_len() {
        let mut a = Aggregator::new(AggregatorConfig { max_text_len: 3, ..cfg(Platform::Windows) });
        for (i, c) in ["a", "b", "c"].iter().enumerate() {
            a.push(&ch(c), i as u64);
        }
        assert_eq!(label(&a.push(&ch("d"), 4).unwrap()), ("new", 2, "d", 1));
    }

    #[test]
    fn counts_repeated_combo() {
        let mut a = Aggregator::new(cfg(Platform::Mac));
        assert_eq!(label(&a.push(&cmd("z"), 0).unwrap()), ("new", 1, "⌘ Z", 1));
        assert_eq!(label(&a.push(&cmd("z"), 300).unwrap()), ("update", 1, "⌘ Z", 2));
        assert_eq!(label(&a.push(&cmd("z"), 600).unwrap()), ("update", 1, "⌘ Z", 3));
        assert_eq!(label(&a.push(&cmd("x"), 700).unwrap()), ("new", 2, "⌘ X", 1));
        assert_eq!(label(&a.push(&cmd("x"), 5000).unwrap()), ("new", 3, "⌘ X", 1));
    }

    #[test]
    fn special_keys_break_typing_and_repeat() {
        let mut a = Aggregator::new(cfg(Platform::Windows));
        a.push(&ch("a"), 0);
        assert_eq!(label(&a.push(&named(NamedKey::Backspace), 10).unwrap()), ("new", 2, "⌫", 1));
        assert_eq!(label(&a.push(&named(NamedKey::Backspace), 20).unwrap()), ("update", 2, "⌫", 2));
        assert_eq!(label(&a.push(&ch("b"), 30).unwrap()), ("new", 3, "b", 1));
    }

    #[test]
    fn ctrl_combo_uses_base_key_even_with_control_text() {
        let mut a = Aggregator::new(cfg(Platform::Windows));
        let ev = KeyEvent::new(
            Key::Char("c".into()),
            Some("\u{3}"),
            Modifiers { ctrl: true, shift: true, ..Modifiers::NONE },
        );
        assert_eq!(label(&a.push(&ev, 0).unwrap()), ("new", 1, "Ctrl + Shift + C", 1));
    }

    #[test]
    fn modifier_only_hidden_by_default() {
        let mut a = Aggregator::new(cfg(Platform::Windows));
        assert!(a.push(&named(NamedKey::Shift), 0).is_none());
        let mut a = Aggregator::new(AggregatorConfig { show_modifier_only: true, ..cfg(Platform::Windows) });
        assert_eq!(label(&a.push(&named(NamedKey::Shift), 0).unwrap()), ("new", 1, "Shift", 1));
    }

    #[test]
    fn modifier_press_does_not_break_typing() {
        let mut a = Aggregator::new(cfg(Platform::Windows));
        a.push(&ch("a"), 0);
        assert!(a.push(&named(NamedKey::Shift), 10).is_none());
        assert_eq!(label(&a.push(&shifted("b", "B"), 20).unwrap()), ("update", 1, "aB", 1));
    }

    #[test]
    fn shortcuts_only_mode() {
        let mut a = Aggregator::new(AggregatorConfig { display_mode: DisplayMode::ShortcutsOnly, ..cfg(Platform::Mac) });
        assert!(a.push(&ch("a"), 0).is_none());
        assert!(a.push(&named(NamedKey::Enter), 10).is_none());
        assert!(a.push(&shifted("a", "A"), 20).is_none());
        assert_eq!(label(&a.push(&cmd("s"), 30).unwrap()), ("new", 1, "⌘ S", 1));
        let shift_tab = KeyEvent::new(Key::Named(NamedKey::Tab), None, Modifiers { shift: true, ..Modifiers::NONE });
        assert_eq!(label(&a.push(&shift_tab, 40).unwrap()), ("new", 2, "⇧ ⇥", 1));
    }

    #[test]
    fn clicks() {
        let mut a = Aggregator::new(cfg(Platform::Windows));
        a.push(&ch("a"), 0);
        assert_eq!(label(&a.push_click(MouseButton::Left, Modifiers::NONE, 10).unwrap()), ("new", 2, "Click", 1));
        assert_eq!(label(&a.push_click(MouseButton::Left, Modifiers::NONE, 200).unwrap()), ("update", 2, "Click", 2));
        let ctrl = Modifiers { ctrl: true, ..Modifiers::NONE };
        assert_eq!(label(&a.push_click(MouseButton::Right, ctrl, 300).unwrap()), ("new", 3, "Ctrl + Right Click", 1));
        // クリックの後の文字入力は新しいアイテム
        assert_eq!(label(&a.push(&ch("b"), 400).unwrap()), ("new", 4, "b", 1));

        let mut s = Aggregator::new(AggregatorConfig { display_mode: DisplayMode::ShortcutsOnly, ..cfg(Platform::Mac) });
        assert!(s.push_click(MouseButton::Left, Modifiers::NONE, 0).is_none());
        let cmd = Modifiers { meta: true, ..Modifiers::NONE };
        assert_eq!(label(&s.push_click(MouseButton::Left, cmd, 10).unwrap()), ("new", 1, "⌘ Click", 1));
    }

    #[test]
    fn scrolls() {
        let mut a = Aggregator::new(cfg(Platform::Windows));
        let none = Modifiers::NONE;
        assert_eq!(label(&a.push_scroll(ScrollDirection::Down, none, 0).unwrap()), ("new", 1, "Scroll ↓", 1));
        // 細かいイベントはまとめる
        assert!(a.push_scroll(ScrollDirection::Down, none, 30).is_none());
        assert!(a.push_scroll(ScrollDirection::Down, none, 60).is_none());
        assert_eq!(label(&a.push_scroll(ScrollDirection::Down, none, 100).unwrap()), ("update", 1, "Scroll ↓", 2));
        // 向きが変わったら新しいアイテム（間隔に関係なく）
        assert_eq!(label(&a.push_scroll(ScrollDirection::Up, none, 110).unwrap()), ("new", 2, "Scroll ↑", 1));
        let ctrl = Modifiers { ctrl: true, ..Modifiers::NONE };
        assert_eq!(label(&a.push_scroll(ScrollDirection::Up, ctrl, 300).unwrap()), ("new", 3, "Ctrl + Scroll ↑", 1));

        let mut s = Aggregator::new(AggregatorConfig { display_mode: DisplayMode::ShortcutsOnly, ..cfg(Platform::Mac) });
        assert!(s.push_scroll(ScrollDirection::Left, none, 0).is_none());
        let cmd = Modifiers { meta: true, ..Modifiers::NONE };
        assert_eq!(label(&s.push_scroll(ScrollDirection::Left, cmd, 10).unwrap()), ("new", 1, "⌘ Scroll ←", 1));
    }

    #[test]
    fn update_serializes_for_frontend() {
        let u = Update::New(DisplayItem { id: 7, kind: DisplayKind::Combo, label: "⌘ S".into(), count: 1 });
        assert_eq!(
            serde_json::to_string(&u).unwrap(),
            r#"{"type":"new","item":{"id":7,"kind":"combo","label":"⌘ S","count":1}}"#
        );
    }
}
