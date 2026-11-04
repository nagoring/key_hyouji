//! アプリ全体の状態と、状態変更（一時停止・表示モード・配置モード）の操作。

use crate::{overlay, tray, OVERLAY};
use key_core::config::{Config, Position, Preset};
use key_core::{Aggregator, DisplayMode};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum HookStatus {
    Starting,
    Running,
    NeedPermission,
    Error { message: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    pub vertical: &'static str,
    pub horizontal: &'static str,
    pub arranging: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub display_duration_ms: u64,
    pub fade_duration_ms: u64,
    pub max_items: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayState {
    pub layout: Layout,
    pub settings: Settings,
    pub paused: bool,
    pub hook: HookStatus,
}

pub struct AppState {
    config_dir: PathBuf,
    pub config: Mutex<Config>,
    pub aggregator: Mutex<Aggregator>,
    paused: AtomicBool,
    arranging: AtomicBool,
    hook: Mutex<HookStatus>,
}

impl AppState {
    pub fn new(config_dir: PathBuf, config: Config, aggregator: Aggregator) -> Self {
        Self {
            config_dir,
            config: Mutex::new(config),
            aggregator: Mutex::new(aggregator),
            paused: AtomicBool::new(false),
            arranging: AtomicBool::new(false),
            hook: Mutex::new(HookStatus::Starting),
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    pub fn is_arranging(&self) -> bool {
        self.arranging.load(Ordering::SeqCst)
    }

    pub fn hook_status(&self) -> HookStatus {
        self.hook.lock().unwrap().clone()
    }

    pub fn layout(&self) -> Layout {
        let align = self.config.lock().unwrap().position.align();
        Layout { vertical: align.vertical(), horizontal: align.horizontal(), arranging: self.is_arranging() }
    }

    pub fn settings(&self) -> Settings {
        let c = self.config.lock().unwrap();
        Settings {
            display_duration_ms: c.display_duration_ms,
            fade_duration_ms: c.fade_duration_ms,
            max_items: c.max_items,
        }
    }

    pub fn overlay_state(&self) -> OverlayState {
        OverlayState {
            layout: self.layout(),
            settings: self.settings(),
            paused: self.is_paused(),
            hook: self.hook_status(),
        }
    }

    /// 設定を変更して保存し、集約ロジックにも反映する。
    fn update_config(&self, f: impl FnOnce(&mut Config)) {
        let mut c = self.config.lock().unwrap();
        f(&mut c);
        c.save(&self.config_dir);
        self.aggregator.lock().unwrap().set_config(c.aggregator_config());
    }
}

fn state(app: &AppHandle) -> tauri::State<'_, AppState> {
    app.state::<AppState>()
}

pub fn toggle_paused(app: &AppHandle) {
    let paused = !state(app).is_paused();
    set_paused(app, paused);
}

pub fn set_paused(app: &AppHandle, paused: bool) {
    let s = state(app);
    s.paused.store(paused, Ordering::SeqCst);
    s.aggregator.lock().unwrap().reset();
    let _ = app.emit_to(OVERLAY, "paused", paused);
    tray::sync(app);
}

pub fn set_display_mode(app: &AppHandle, mode: DisplayMode) {
    state(app).update_config(|c| c.display_mode = mode);
    tray::sync(app);
}

pub fn set_preset(app: &AppHandle, preset: Preset) {
    let s = state(app);
    if s.is_arranging() {
        s.arranging.store(false, Ordering::SeqCst);
        if let Some(w) = app.get_webview_window(OVERLAY) {
            let _ = w.set_ignore_cursor_events(true);
        }
    }
    s.update_config(|c| c.position = Position::Preset { preset });
    if let Err(e) = overlay::apply_position(app) {
        log::error!("failed to move overlay: {e}");
    }
    let _ = app.emit_to(OVERLAY, "layout", s.layout());
    tray::sync(app);
}

/// 配置モード（W-04）。オン: ドラッグで移動可能。オフ: 現在位置を保存してクリック透過に戻す。
pub fn set_arranging(app: &AppHandle, on: bool) {
    let s = state(app);
    let Some(window) = app.get_webview_window(OVERLAY) else { return };
    s.arranging.store(on, Ordering::SeqCst);
    let _ = window.set_ignore_cursor_events(!on);
    if on {
        let _ = window.set_focus();
    } else if let Ok(p) = window.outer_position() {
        s.update_config(|c| {
            let align = c.position.align();
            c.position = Position::Custom { x: p.x, y: p.y, align };
        });
    }
    let _ = app.emit_to(OVERLAY, "layout", s.layout());
    tray::sync(app);
}

pub fn set_hook_status(app: &AppHandle, status: HookStatus) {
    *state(app).hook.lock().unwrap() = status.clone();
    let _ = app.emit_to(OVERLAY, "hook-status", status);
    tray::sync(app);
}
