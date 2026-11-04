// リリースビルドで Windows のコンソールウィンドウを出さない
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod overlay;
mod state;
mod tray;
mod windows;

use key_core::config::Config;
use key_core::hook::{self, HookError};
use key_core::{Aggregator, KeyEvent};
use state::{AppState, HookStatus};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::ShortcutState;

pub const OVERLAY: &str = "overlay";

fn main() {
    tauri::Builder::default()
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
            commands::restart_app,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.handle().set_activation_policy(tauri::ActivationPolicy::Accessory)?;

            let config_dir = app.path().app_config_dir()?;
            let config = Config::load(&config_dir).sanitized();
            let aggregator = Aggregator::new(config.aggregator_config());
            app.manage(AppState::new(config_dir, config, aggregator));

            let window = app
                .get_webview_window(OVERLAY)
                .expect("overlay window is defined in tauri.conf.json");
            // 背後のアプリにマウス操作を通す（W-02）
            window.set_ignore_cursor_events(true)?;
            overlay::apply_position(app.handle())?;
            window.show()?;

            tray::create(app.handle())?;
            state::register_initial_hotkey(app.handle());
            overlay::spawn_follow(app.handle().clone());

            // macOS: 入力監視の権限が無ければ案内画面を出す
            if !hook::has_permission() {
                windows::open_onboarding(app.handle());
            }

            let (tx, rx) = channel::<KeyEvent>();
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
fn spawn_pipeline(app: AppHandle, rx: Receiver<KeyEvent>) {
    thread::Builder::new()
        .name("key-pipeline".into())
        .spawn(move || {
            let started = Instant::now();
            for ev in rx {
                let state = app.state::<AppState>();
                if state.is_paused() {
                    continue;
                }
                if state.foreground_excluded() {
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

/// キーフックを開始する。macOS で権限が無い場合は許可されるまで再試行する。
fn spawn_hook(app: AppHandle, tx: Sender<KeyEvent>) {
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
