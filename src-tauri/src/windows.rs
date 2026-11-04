//! 設定画面・オンボーディング画面の表示。

use crate::state::AppState;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub const SETTINGS: &str = "settings";
pub const ONBOARDING: &str = "onboarding";

fn open(app: &AppHandle, label: &str, url: &str, title: &str, size: (f64, f64), resizable: bool) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let result = WebviewWindowBuilder::new(app, label, WebviewUrl::App(url.into()))
        .title(title)
        .inner_size(size.0, size.1)
        .min_inner_size(size.0.min(560.0), size.1.min(420.0))
        .resizable(resizable)
        .center()
        .focused(true)
        .build();
    match result {
        Ok(w) => {
            let _ = w.set_focus();
        }
        Err(e) => log::error!("failed to open {label} window: {e}"),
    }
}

pub fn open_settings(app: &AppHandle) {
    let title = app.state::<AppState>().strings().settings_title;
    open(app, SETTINGS, "settings.html", title, (760.0, 600.0), true);
}

pub fn open_onboarding(app: &AppHandle) {
    let title = app.state::<AppState>().strings().onboarding_title;
    open(app, ONBOARDING, "onboarding.html", title, (560.0, 540.0), false);
}

/// 言語変更時に開いているウィンドウのタイトルを更新する。
pub fn retitle(app: &AppHandle) {
    let tr = app.state::<AppState>().strings();
    if let Some(w) = app.get_webview_window(SETTINGS) {
        let _ = w.set_title(tr.settings_title);
    }
    if let Some(w) = app.get_webview_window(ONBOARDING) {
        let _ = w.set_title(tr.onboarding_title);
    }
}

pub fn open_privacy_settings() {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent")
            .spawn();
    }
}
