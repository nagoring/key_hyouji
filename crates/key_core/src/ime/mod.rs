//! IME（日本語入力など）がオンかの検知（F-11）。
//!
//! - Windows: 最前面ウィンドウの IME が開いていて、ネイティブ入力モード（ひらがな等）か
//! - macOS: 現在の入力ソースが入力メソッドのモードで、ローマ字（英数）モードでないか
//!
//! macOS の Text Input Sources API はメインスレッドから呼ぶ必要がある。

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
    pub fn ime_active() -> bool {
        false
    }
}

/// IME で文字を変換する入力モードか。
pub fn ime_active() -> bool {
    imp::ime_active()
}

/// macOS の入力ソース ID と種類から IME がオンか判定する（テスト用に分離）。
pub fn is_ime_source(source_id: &str, is_input_mode: bool) -> bool {
    is_input_mode && !source_id.ends_with(".Roman") && !source_id.ends_with(".ABC")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_source_ids() {
        assert!(is_ime_source("com.apple.inputmethod.Japanese.Hiragana", true));
        assert!(is_ime_source("com.google.inputmethod.Japanese.base", true));
        assert!(!is_ime_source("com.apple.inputmethod.Japanese.Roman", true));
        assert!(!is_ime_source("com.google.inputmethod.Japanese.Roman", true));
        assert!(!is_ime_source("com.apple.keylayout.ABC", false));
        assert!(!is_ime_source("com.apple.keylayout.US", false));
    }
}
