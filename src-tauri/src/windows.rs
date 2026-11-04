//! 設定画面・オンボーディング画面の表示。

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
    open(app, SETTINGS, "settings.html", "key_hyouji 設定", (760.0, 600.0), true);
}

pub fn open_onboarding(app: &AppHandle) {
    open(app, ONBOARDING, "onboarding.html", "key_hyouji へようこそ", (560.0, 520.0), false);
}

pub fn open_privacy_settings() {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent")
            .spawn();
    }
}
