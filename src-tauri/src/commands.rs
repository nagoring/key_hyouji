//! フロントエンド（オーバーレイ / 設定 / オンボーディング）から呼ぶコマンド。

use crate::overlay::{self, MonitorInfo};
use crate::state::{self, AppState, HookStatus, OverlayState};
use crate::{windows, OVERLAY};
use key_core::config::{Config, Position};
use key_core::{hook, DisplayItem, DisplayKind, Platform, Update};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

#[tauri::command]
pub fn get_overlay_state(app: AppHandle) -> OverlayState {
    app.state::<AppState>().overlay_state()
}

#[tauri::command]
pub fn get_config(app: AppHandle) -> Config {
    app.state::<AppState>().config()
}

#[tauri::command]
pub fn set_config(app: AppHandle, config: Config) -> Result<Config, String> {
    state::apply_config(&app, config)
}

/// 初期設定に戻す（表示位置とモニターは維持）。
#[tauri::command]
pub fn reset_config(app: AppHandle) -> Result<Config, String> {
    let current = app.state::<AppState>().config();
    let config = Config { position: current.position, monitor: current.monitor, ..Config::default() };
    state::apply_config(&app, config)
}

#[tauri::command]
pub fn list_monitors(app: AppHandle) -> Vec<MonitorInfo> {
    overlay::list_monitors(&app)
}

#[tauri::command]
pub fn list_running_apps() -> Vec<String> {
    key_core::foreground::running_apps()
}

#[tauri::command]
pub fn set_arranging(app: AppHandle, on: bool) {
    state::set_arranging(&app, on);
}

#[tauri::command]
pub fn is_arranging(app: AppHandle) -> bool {
    app.state::<AppState>().is_arranging()
}

/// 表示位置をプリセットに戻す（配置モードで決めた位置を破棄）。
#[tauri::command]
pub fn reset_position(app: AppHandle) -> Result<Config, String> {
    let mut c = app.state::<AppState>().config();
    c.position = Position::Preset { preset: c.position.align() };
    state::apply_config(&app, c)
}

/// 外観確認用のサンプルをオーバーレイに表示する。
#[tauri::command]
pub fn show_preview(app: AppHandle) {
    let mac = Platform::current() == Platform::Mac;
    let samples = [
        (DisplayKind::Text, "Hello", 1),
        (DisplayKind::Combo, if mac { "⌘ ⇧ S" } else { "Ctrl + Shift + S" }, 1),
        (DisplayKind::Combo, if mac { "⌘ Z" } else { "Ctrl + Z" }, 3),
    ];
    // 実際のキー表示と衝突しない ID を使う
    let base = u64::MAX / 2;
    let max = app.state::<AppState>().config().max_items as usize;
    for (i, (kind, label, count)) in samples.into_iter().enumerate().take(max.max(1)) {
        let item = DisplayItem { id: base + i as u64, kind, label: label.into(), count };
        let _ = app.emit_to(OVERLAY, "keystroke", Update::New(item));
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    version: String,
    platform: Platform,
    config_path: String,
}

#[tauri::command]
pub fn get_app_info(app: AppHandle) -> AppInfo {
    AppInfo {
        version: app.package_info().version.to_string(),
        platform: Platform::current(),
        config_path: app.state::<AppState>().config_path().display().to_string(),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionStatus {
    granted: bool,
    hook: HookStatus,
}

#[tauri::command]
pub fn get_permission_status(app: AppHandle) -> PermissionStatus {
    PermissionStatus { granted: hook::has_permission(), hook: app.state::<AppState>().hook_status() }
}

#[tauri::command]
pub fn request_permission() {
    hook::request_permission();
}

#[tauri::command]
pub fn open_privacy_settings() {
    windows::open_privacy_settings();
}

#[tauri::command]
pub fn restart_app(app: AppHandle) {
    app.restart();
}
