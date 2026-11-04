//! フォアグラウンド（最前面）アプリの取得。除外アプリ（P-03）と
//! 「アクティブウィンドウのモニター」（W-05）で使う。

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
    pub fn foreground_app() -> Option<super::ForegroundApp> {
        None
    }
    pub fn running_apps() -> Vec<String> {
        Vec::new()
    }
}

/// ウィンドウの矩形。
/// 座標系: Windows は物理ピクセル、macOS はポイント（論理ピクセル）。どちらもメイン画面左上が原点。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn center(&self) -> (f64, f64) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForegroundApp {
    /// Windows: 実行ファイル名（例: `notepad.exe`）/ macOS: アプリ名（例: `Safari`）
    pub name: String,
    /// このアプリ自身（設定画面など）か
    pub is_self: bool,
    pub bounds: Option<Rect>,
}

pub fn foreground_app() -> Option<ForegroundApp> {
    imp::foreground_app()
}

/// ウィンドウを持つ実行中アプリの名前（重複なし・ソート済み・自分自身を除く）。
pub fn running_apps() -> Vec<String> {
    let mut apps = imp::running_apps();
    apps.sort_by_key(|a| a.to_lowercase());
    apps.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    apps
}
