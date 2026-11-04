//! OS ごとのグローバルキーフック。
//!
//! どの OS でも「キーダウン（修飾キーは押下時）」だけを [`KeyEvent`] として送る。
//! 受け取ったキー入力はチャネルに流すだけで、保存・送信は一切しない（P-01）。

use crate::event::KeyEvent;
use std::fmt;
use std::sync::mpsc::Sender;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as imp;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use self::windows as imp;

#[cfg(not(any(windows, target_os = "macos")))]
mod unsupported;
#[cfg(not(any(windows, target_os = "macos")))]
use unsupported as imp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookError {
    /// macOS の「入力監視」権限が無い
    PermissionDenied,
    Failed(String),
}

impl fmt::Display for HookError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HookError::PermissionDenied => write!(f, "input monitoring permission is not granted"),
            HookError::Failed(s) => write!(f, "failed to install keyboard hook: {s}"),
        }
    }
}

impl std::error::Error for HookError {}

/// フック用スレッドを起動し、インストール結果を返す。
///
/// 失敗した場合はスレッドは残らないので、権限付与後などに再度呼び出せる。
/// 成功後に再度呼ぶとエラーになる。
pub fn start(sink: Sender<KeyEvent>) -> Result<(), HookError> {
    imp::start(sink)
}

/// キー入力の取得に必要な権限があるか（macOS 以外は常に true）。
pub fn has_permission() -> bool {
    imp::has_permission()
}

/// 権限をリクエストする（macOS ではシステムのダイアログを表示）。
pub fn request_permission() {
    imp::request_permission()
}
