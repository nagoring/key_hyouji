// リリースビルドで Windows のコンソールウィンドウを出さない
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod i18n;
mod overlay;
mod ring;
mod state;
mod tray;
mod windows;

use key_core::config::{Config, ImeMode};
use key_core::hook::{self, HookError};
use key_core::aggregator::is_typing;
use key_core::{Aggregator, DisplayItem, DisplayKind, InputEvent, Update};
use state::{AppState, HookStatus};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::ShortcutState;

pub const OVERLAY: &str = "overlay";

/// パスワード入力中に出す「🔒」表示の ID（通常のキー表示と衝突しない値）
const SECURE_ITEM_ID: u64 = u64::MAX - 1;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    // 登録するのは一時停止ホットキーのみ（C-02）
                    if event.state == ShortcutState::Pressed {
                        state::toggle_paused(app);
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::get_overlay_state,
            commands::get_config,
            commands::set_config,
            commands::reset_config,
            commands::list_monitors,
            commands::list_running_apps,
            commands::set_arranging,
            commands::is_arranging,
            commands::reset_position,
            commands::show_preview,
            commands::get_app_info,
            commands::get_permission_status,
            commands::request_permission,
            commands::open_privacy_settings,
            commands::get_language,
            commands::export_config,
            commands::import_config,
            commands::restart_app,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.handle().set_activation_policy(tauri::ActivationPolicy::Accessory)?;

            let config_dir = app.path().app_config_dir()?;
            let config = Config::load(&config_dir).sanitized();
            let mouse_enabled = config.mouse.any_enabled();
            let state = AppState::new(config_dir, config.clone(), Aggregator::new(config.aggregator_config()));
            state.aggregator.lock().unwrap().set_config(state.aggregator_config(&config));
            app.manage(state);
            key_core::hook::set_mouse_enabled(mouse_enabled);

            let window = app
                .get_webview_window(OVERLAY)
                .expect("overlay window is defined in tauri.conf.json");
            // 背後のアプリにマウス操作を通す（W-02）
            window.set_ignore_cursor_events(true)?;
            overlay::apply_position(app.handle())?;
            window.show()?;

            tray::create(app.handle())?;
            if let Err(e) = ring::create(app.handle()) {
                log::warn!("failed to create click ring window: {e}");
            }
            state::register_initial_hotkey(app.handle());
            overlay::spawn_follow(app.handle().clone());
            spawn_secure_input_watch(app.handle().clone());

            // 自動起動が有効なら登録先を現在の実行ファイルに合わせる（exe を移動した場合への追従）
            let state = app.state::<AppState>();
            if state.config().auto_start {
                if let Ok(exe) = std::env::current_exe() {
                    if let Err(e) = key_core::autostart::set_enabled(true, &exe) {
                        log::warn!("failed to refresh autostart: {e}");
                    }
                }
            }

            // macOS: 入力監視の権限が無ければ案内画面を出す
            if !hook::has_permission() {
                windows::open_onboarding(app.handle());
            }

            let (tx, rx) = channel::<InputEvent>();
            spawn_pipeline(app.handle().clone(), rx);
            spawn_hook(app.handle().clone(), tx);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building key_hyouji")
        .run(|_app, event| {
            // 設定画面などを閉じてもトレイ常駐を続ける
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}

/// フックから届いたキーイベントを表示アイテムに変換してオーバーレイへ送る。
fn spawn_pipeline(app: AppHandle, rx: Receiver<InputEvent>) {
    thread::Builder::new()
        .name("key-pipeline".into())
        .spawn(move || {
            let started = Instant::now();
            for input in rx {
                let state = app.state::<AppState>();
                if state.is_paused() {
                    continue;
                }
                if state.foreground_excluded() {
                    state.aggregator.lock().unwrap().reset();
                    continue;
                }
                let ev = match input {
                    InputEvent::Key(ev) => ev,
                    InputEvent::Mouse(m) => {
                        let mouse = state.config().mouse;
                        if mouse.show_clicks {
                            let now_ms = started.elapsed().as_millis() as u64;
                            let update = state.aggregator.lock().unwrap().push_click(m.button, m.modifiers, now_ms);
                            if let Some(update) = update {
                                let _ = app.emit_to(OVERLAY, "keystroke", update);
                            }
                        }
                        if mouse.show_ring {
                            ring::show(&app, &m);
                        }
                        continue;
                    }
                    InputEvent::Scroll(sc) => {
                        if state.config().mouse.show_scroll {
                            let now_ms = started.elapsed().as_millis() as u64;
                            let update = state.aggregator.lock().unwrap().push_scroll(sc.direction, sc.modifiers, now_ms);
                            if let Some(update) = update {
                                let _ = app.emit_to(OVERLAY, "keystroke", update);
                            }
                        }
                        continue;
                    }
                };
                let (mask_password, show_indicator) = {
                    let c = state.config.lock().unwrap();
                    (c.mask_password_fields, c.show_secure_indicator)
                };
                // パスワード欄への入力は表示しない（P-02）。回数も出さない
                if mask_password && key_core::secure::password_field_focused() {
                    state.aggregator.lock().unwrap().reset();
                    if show_indicator {
                        let item = DisplayItem { id: SECURE_ITEM_ID, kind: DisplayKind::Combo, label: "🔒".into(), count: 1 };
                        let _ = app.emit_to(OVERLAY, "keystroke", Update::Update(item));
                    }
                    continue;
                }
                // IME オン中の文字入力（変換前のローマ字）は表示しない（F-11）
                if state.ime_mode() == ImeMode::Hide && is_typing(&ev) && state.ime_active(&app) {
                    state.aggregator.lock().unwrap().reset();
                    continue;
                }
                let now_ms = started.elapsed().as_millis() as u64;
                let update = state.aggregator.lock().unwrap().push(&ev, now_ms);
                if let Some(update) = update {
                    let _ = app.emit_to(OVERLAY, "keystroke", update);
                }
            }
        })
        .expect("spawn pipeline thread");
}

/// macOS の Secure Event Input（パスワード入力中など）を監視し、オーバーレイに知らせる。
fn spawn_secure_input_watch(app: AppHandle) {
    if !cfg!(target_os = "macos") {
        return;
    }
    thread::Builder::new()
        .name("secure-input-watch".into())
        .spawn(move || {
            let mut last = false;
            loop {
                thread::sleep(Duration::from_millis(500));
                let enabled = app.state::<AppState>().config().show_secure_indicator
                    && key_core::secure::secure_input_enabled();
                if enabled != last {
                    last = enabled;
                    let _ = app.emit_to(OVERLAY, "secure-input", enabled);
                }
            }
        })
        .expect("spawn secure input thread");
}

/// キーフックを開始する。macOS で権限が無い場合は許可されるまで再試行する。
fn spawn_hook(app: AppHandle, tx: Sender<InputEvent>) {
    thread::Builder::new()
        .name("key-hook-starter".into())
        .spawn(move || {
            if !hook::has_permission() {
                hook::request_permission();
            }
            loop {
                match hook::start(tx.clone()) {
                    Ok(()) => {
                        state::set_hook_status(&app, HookStatus::Running);
                        break;
                    }
                    Err(HookError::PermissionDenied) => {
                        state::set_hook_status(&app, HookStatus::NeedPermission);
                        thread::sleep(Duration::from_secs(2));
                    }
                    Err(e) => {
                        log::error!("{e}");
                        state::set_hook_status(&app, HookStatus::Error { message: e.to_string() });
                        break;
                    }
                }
            }
        })
        .expect("spawn hook thread");
}
