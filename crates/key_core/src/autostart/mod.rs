//! OS 起動（ログイン）時の自動起動（C-04）。レジストリは使わない。
//!
//! - Windows: スタートアップフォルダ（`shell:startup`）にショートカット（.lnk）を作る
//! - macOS: `~/Library/LaunchAgents` に LaunchAgent の plist を置く

use std::io;
use std::path::Path;

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
    use std::io;
    use std::path::Path;
    pub fn set_enabled(_enabled: bool, _exe: &Path) -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::Unsupported, "autostart is not supported on this platform"))
    }
    pub fn is_enabled() -> bool {
        false
    }
}

pub const LABEL: &str = "com.nagoring.keyhyouji";

/// 自動起動を有効 / 無効にする。有効化は `exe` を起動するよう登録し直す
/// （ポータブル版で exe を移動した場合も起動時に呼べば追従する）。
pub fn set_enabled(enabled: bool, exe: &Path) -> io::Result<()> {
    imp::set_enabled(enabled, exe)
}

pub fn is_enabled() -> bool {
    imp::is_enabled()
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// macOS LaunchAgent の plist（テスト用に OS 非依存で公開）。
pub fn launch_agent_plist(label: &str, exe: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{label}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{exe}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>LimitLoadToSessionType</key>
  <string>Aqua</string>
  <key>ProcessType</key>
  <string>Interactive</string>
</dict>
</plist>
"#,
        label = xml_escape(label),
        exe = xml_escape(exe)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plist_escapes_path() {
        let p = launch_agent_plist(LABEL, "/Applications/A & B/key_hyouji.app/Contents/MacOS/key_hyouji");
        assert!(p.contains("<string>/Applications/A &amp; B/key_hyouji.app/Contents/MacOS/key_hyouji</string>"));
        assert!(p.contains("<key>RunAtLoad</key>\n  <true/>"));
    }
}
