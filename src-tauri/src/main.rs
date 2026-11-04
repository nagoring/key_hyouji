// リリースビルドで Windows のコンソールウィンドウを出さない
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod overlay;
mod state;
mod tray;

use key_core::config::Config;
use key_core::hook::{self, HookError};
use key_core::{Aggregator, KeyEvent};
use state::{AppState, HookStatus, OverlayState};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

pub const OVERLAY: &str = "overlay";

fn pause_shortcut() -> Shortcut {
    // 仕様 C-02: Ctrl+Alt+Shift+K / ⌃⌥⇧K
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT), Code::KeyK)
}

#[tauri::command]
fn get_overlay_state(app: AppHandle) -> OverlayState {
    app.state::<AppState>().overlay_state()
}

fn main() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state == ShortcutState::Pressed && *shortcut == pause_shortcut() {
                        state::toggle_paused(app);
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![get_overlay_state])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.handle().set_activation_policy(tauri::ActivationPolicy::Accessory)?;

            let config_dir = app.path().app_config_dir()?;
            let config = Config::load(&config_dir);
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

            if let Err(e) = app.global_shortcut().register(pause_shortcut()) {
                log::warn!("failed to register pause shortcut: {e}");
            }

            let (tx, rx) = channel::<KeyEvent>();
            spawn_pipeline(app.handle().clone(), rx);
            spawn_hook(app.handle().clone(), tx);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running key_hyouji");
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
