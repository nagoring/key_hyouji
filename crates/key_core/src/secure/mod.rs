//! パスワード入力など「安全な入力」の検知（P-02）。
//!
//! - Windows: UI Automation でフォーカス中の要素が `IsPassword` か調べる
//! - macOS: Secure Event Input が有効か（有効な間は OS がキーイベントを渡さない）

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as imp;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use self::windows as imp;

#[cfg(not(any(windows, target_os = "macos")))]
mod imp {
    pub fn password_field_focused() -> bool {
        false
    }
    pub fn secure_input_enabled() -> bool {
        false
    }
}

/// フォーカス中の入力欄がパスワード欄か（Windows のみ。他 OS は常に false）。
///
/// COM を使うため、呼び出したスレッドで COM を初期化する（以降も同じスレッドで呼ぶこと）。
pub fn password_field_focused() -> bool {
    imp::password_field_focused()
}

/// macOS の Secure Event Input が有効か（macOS のみ。他 OS は常に false）。
pub fn secure_input_enabled() -> bool {
    imp::secure_input_enabled()
}
