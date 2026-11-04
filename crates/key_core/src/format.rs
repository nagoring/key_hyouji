use crate::event::{Key, Modifiers, NamedKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Windows,
    Mac,
}

impl Platform {
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Platform::Mac
        } else {
            Platform::Windows
        }
    }
}

/// 修飾キーの表記スタイル（F-08）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModifierStyle {
    /// Mac: `⌃ ⌥ ⇧ ⌘` / Windows: `Ctrl + Alt + Shift + Win`
    Symbol,
    /// Mac: `Control + Option + Shift + Command` / Windows: `Ctrl + Alt + Shift + Win`
    Text,
}

/// 組み合わせ（ショートカット）表示用の文字列を作る。
///
/// 並び順は各 OS の慣例に合わせる。
/// - Mac: ⌃ ⌥ ⇧ ⌘ （スペース区切り）
/// - Windows: Ctrl + Alt + Shift + Win
pub fn combo_label(key: &Key, m: Modifiers, platform: Platform, style: ModifierStyle) -> String {
    let mut parts: Vec<&str> = Vec::with_capacity(5);
    match (platform, style) {
        (Platform::Mac, ModifierStyle::Symbol) => {
            if m.ctrl { parts.push("⌃"); }
            if m.alt { parts.push("⌥"); }
            if m.shift { parts.push("⇧"); }
            if m.meta { parts.push("⌘"); }
        }
        (Platform::Mac, ModifierStyle::Text) => {
            if m.ctrl { parts.push("Control"); }
            if m.alt { parts.push("Option"); }
            if m.shift { parts.push("Shift"); }
            if m.meta { parts.push("Command"); }
        }
        (Platform::Windows, _) => {
            if m.ctrl { parts.push("Ctrl"); }
            if m.alt { parts.push("Alt"); }
            if m.shift { parts.push("Shift"); }
            if m.meta { parts.push("Win"); }
        }
    }
    let key_label = key_label(key, platform);
    parts.push(&key_label);
    let sep = match (platform, style) {
        (Platform::Mac, ModifierStyle::Symbol) => " ",
        _ => " + ",
    };
    parts.join(sep)
}

/// キー単体の表示名（F-07）。
pub fn key_label(key: &Key, platform: Platform) -> String {
    match key {
        Key::Char(c) => c.to_uppercase(),
        Key::Unknown(s) => s.clone(),
        Key::Named(n) => named_label(*n, platform).into_owned(),
    }
}

fn named_label(n: NamedKey, platform: Platform) -> std::borrow::Cow<'static, str> {
    use std::borrow::Cow::Borrowed;
    let mac = platform == Platform::Mac;
    let s = match n {
        NamedKey::Enter => "⏎",
        NamedKey::Tab => "⇥",
        NamedKey::Space => "␣",
        NamedKey::Backspace => "⌫",
        NamedKey::Delete => "⌦",
        NamedKey::Escape => "Esc",
        NamedKey::Left => "←",
        NamedKey::Right => "→",
        NamedKey::Up => "↑",
        NamedKey::Down => "↓",
        NamedKey::Home => "Home",
        NamedKey::End => "End",
        NamedKey::PageUp => "PgUp",
        NamedKey::PageDown => "PgDn",
        NamedKey::Insert => "Ins",
        NamedKey::CapsLock => "⇪",
        NamedKey::PrintScreen => "PrtSc",
        NamedKey::F(i) => return std::borrow::Cow::Owned(format!("F{i}")),
        NamedKey::Shift => if mac { "⇧" } else { "Shift" },
        NamedKey::Control => if mac { "⌃" } else { "Ctrl" },
        NamedKey::Alt => if mac { "⌥" } else { "Alt" },
        NamedKey::Meta => if mac { "⌘" } else { "Win" },
        NamedKey::Eisu => "英数",
        NamedKey::Kana => "かな",
        NamedKey::Henkan => "変換",
        NamedKey::Muhenkan => "無変換",
        NamedKey::HankakuZenkaku => "半角/全角",
    };
    Borrowed(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(ctrl: bool, alt: bool, shift: bool, meta: bool) -> Modifiers {
        Modifiers { ctrl, alt, shift, meta }
    }

    #[test]
    fn windows_combo() {
        let k = Key::Char("s".into());
        assert_eq!(
            combo_label(&k, m(true, false, true, false), Platform::Windows, ModifierStyle::Symbol),
            "Ctrl + Shift + S"
        );
    }

    #[test]
    fn mac_combo_symbol_order() {
        let k = Key::Char("s".into());
        assert_eq!(
            combo_label(&k, m(true, true, true, true), Platform::Mac, ModifierStyle::Symbol),
            "⌃ ⌥ ⇧ ⌘ S"
        );
    }

    #[test]
    fn mac_combo_text() {
        let k = Key::Char("z".into());
        assert_eq!(
            combo_label(&k, m(false, false, false, true), Platform::Mac, ModifierStyle::Text),
            "Command + Z"
        );
    }

    #[test]
    fn named_keys() {
        assert_eq!(key_label(&Key::Named(NamedKey::Enter), Platform::Windows), "⏎");
        assert_eq!(key_label(&Key::Named(NamedKey::F(12)), Platform::Mac), "F12");
        assert_eq!(
            combo_label(&Key::Named(NamedKey::Left), m(false, true, false, false), Platform::Windows, ModifierStyle::Symbol),
            "Alt + ←"
        );
    }
}
