use serde::Serialize;

/// 押下時点の修飾キー状態。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    /// Windows キー / ⌘ Command
    pub meta: bool,
}

impl Modifiers {
    pub const NONE: Modifiers = Modifiers { ctrl: false, alt: false, shift: false, meta: false };

    /// Ctrl / Alt / Meta のいずれかが押されているか（Shift 単独は含まない）。
    pub fn has_command(&self) -> bool {
        self.ctrl || self.alt || self.meta
    }

    pub fn any(&self) -> bool {
        self.has_command() || self.shift
    }
}

/// 名前付き（非文字）キー。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum NamedKey {
    Enter,
    Tab,
    Space,
    Backspace,
    Delete,
    Escape,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    CapsLock,
    PrintScreen,
    /// F1..F24
    F(u8),
    // 修飾キー単体
    Shift,
    Control,
    Alt,
    Meta,
    // 日本語キーボード
    Eisu,
    Kana,
    Henkan,
    Muhenkan,
    HankakuZenkaku,
}

impl NamedKey {
    pub fn is_modifier(&self) -> bool {
        matches!(self, NamedKey::Shift | NamedKey::Control | NamedKey::Alt | NamedKey::Meta)
    }
}

/// 物理キーの識別。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Key {
    /// 修飾なしで入力される文字（例: "a", "1", "/"）。
    Char(String),
    Named(NamedKey),
    /// 表示名が分からないキー（OS のキーコードを文字列化したもの等）。
    Unknown(String),
}

/// フックから届くキー押下イベント（キーダウンのみ）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEvent {
    pub key: Key,
    /// 実際に入力される文字列（Shift / CapsLock / 配列を反映）。入力文字が無いキーは `None`。
    pub text: Option<String>,
    pub modifiers: Modifiers,
}

impl KeyEvent {
    pub fn new(key: Key, text: Option<&str>, modifiers: Modifiers) -> Self {
        Self { key, text: text.map(str::to_owned), modifiers }
    }
}

/// マウスボタン（F-10）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// マウスボタンの押下。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseEvent {
    pub button: MouseButton,
    /// 押した位置。Windows は物理ピクセル、macOS はポイント（どちらもメイン画面左上が原点）
    pub x: f64,
    pub y: f64,
    pub modifiers: Modifiers,
}

/// ホイールの回転方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

impl ScrollDirection {
    pub fn arrow(self) -> &'static str {
        match self {
            ScrollDirection::Up => "↑",
            ScrollDirection::Down => "↓",
            ScrollDirection::Left => "←",
            ScrollDirection::Right => "→",
        }
    }
}

/// ホイールの回転（スクロール）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollEvent {
    pub direction: ScrollDirection,
    pub modifiers: Modifiers,
}

/// フックから届く入力。
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Scroll(ScrollEvent),
}
