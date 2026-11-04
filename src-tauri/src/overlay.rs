//! オーバーレイウィンドウの大きさ・位置（W-04）と表示モニター（W-05）。

use crate::state::AppState;
use crate::OVERLAY;
use key_core::config::{Config, MonitorTarget, Position, Preset};
use key_core::foreground;
use serde::Serialize;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager, Monitor, PhysicalPosition, PhysicalSize};

/// 追従モード（カーソル / アクティブウィンドウ）でモニターを調べる間隔
const FOLLOW_INTERVAL: Duration = Duration::from_millis(400);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub primary: bool,
}

pub fn list_monitors(app: &AppHandle) -> Vec<MonitorInfo> {
    let primary = app.primary_monitor().ok().flatten().and_then(|m| m.name().cloned());
    app.available_monitors()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|m| {
            let name = m.name()?.clone();
            let size = m.size();
            Some(MonitorInfo {
                primary: primary.as_deref() == Some(name.as_str()),
                name,
                width: size.width,
                height: size.height,
            })
        })
        .collect()
}

fn contains(m: &Monitor, x: f64, y: f64, logical: bool) -> bool {
    let scale = if logical { m.scale_factor() } else { 1.0 };
    let p = m.position();
    let s = m.size();
    let (mx, my) = (p.x as f64 / scale, p.y as f64 / scale);
    let (mw, mh) = (s.width as f64 / scale, s.height as f64 / scale);
    x >= mx && x < mx + mw && y >= my && y < my + mh
}

/// 設定に従って表示先モニターを決める。見つからなければメインモニター。
pub fn resolve_monitor(app: &AppHandle, target: &MonitorTarget) -> Option<Monitor> {
    let monitors = app.available_monitors().ok()?;
    let primary = app.primary_monitor().ok().flatten().or_else(|| monitors.first().cloned());
    let found = match target {
        MonitorTarget::Primary => None,
        MonitorTarget::Fixed { name } => monitors.iter().find(|m| m.name() == Some(name)).cloned(),
        MonitorTarget::Cursor => app
            .cursor_position()
            .ok()
            .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten()),
        MonitorTarget::Active => foreground::foreground_app()
            .and_then(|f| f.bounds)
            .and_then(|b| {
                let (cx, cy) = b.center();
                // macOS のウィンドウ座標はポイント（論理）、Windows は物理ピクセル
                let logical = cfg!(target_os = "macos");
                monitors.iter().find(|m| contains(m, cx, cy, logical)).cloned()
            }),
    };
    found.or(primary)
}

/// テーマと表示件数から必要なウィンドウの大きさ（論理ピクセル）を求める。
fn logical_size(config: &Config, monitor: &Monitor) -> (f64, f64) {
    let font = config.theme.font_size;
    // 1 件分: 行の高さ 1.3em + 上下パディング 0.5em + 間隔 10px
    let item = font * 1.8 + 10.0;
    let height = (config.max_items as f64 * item + 48.0).max(120.0);
    let scale = monitor.scale_factor();
    let work_w = monitor.work_area().size.width as f64 / scale;
    let width = (font * 30.0).max(600.0).min((work_w - 2.0 * config.offset_x).max(300.0));
    (width, height)
}

fn physical_size(config: &Config, m: &Monitor) -> PhysicalSize<u32> {
    let (w, h) = logical_size(config, m);
    let s = m.scale_factor();
    PhysicalSize::new((w * s).round() as u32, (h * s).round() as u32)
}

pub fn apply_position(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(OVERLAY) else { return Ok(()) };
    let config = app.state::<AppState>().config();
    let monitors = window.available_monitors()?;

    let (monitor, origin) = match config.position {
        // ドラッグで配置した位置は絶対座標（モニター設定より優先）
        Position::Custom { x, y, .. } => {
            let found = monitors
                .iter()
                .find(|m| contains(m, x as f64 + 50.0, y as f64 + 50.0, false))
                .cloned();
            match found {
                Some(m) => (m, (x, y)),
                // 保存位置のモニターが無くなった場合は既定位置へ
                None => {
                    let Some(m) = resolve_monitor(app, &MonitorTarget::Primary) else { return Ok(()) };
                    let o = preset_origin(&config, &m, Preset::BottomCenter);
                    (m, o)
                }
            }
        }
        Position::Preset { preset } => {
            let Some(m) = resolve_monitor(app, &config.monitor) else { return Ok(()) };
            let o = preset_origin(&config, &m, preset);
            (m, o)
        }
    };
    window.set_size(physical_size(&config, &monitor))?;
    window.set_position(PhysicalPosition::new(origin.0, origin.1))?;
    Ok(())
}

/// プリセット位置でのウィンドウ左上座標（タスクバー / Dock を除いた作業領域基準）。
fn preset_origin(config: &Config, m: &Monitor, preset: Preset) -> (i32, i32) {
    let s = m.scale_factor();
    let area = m.work_area();
    let size = physical_size(config, m);
    let (ax, ay) = (area.position.x, area.position.y);
    let (aw, ah) = (area.size.width as i32, area.size.height as i32);
    let (w, h) = (size.width as i32, size.height as i32);
    let ox = (config.offset_x * s).round() as i32;
    let oy = (config.offset_y * s).round() as i32;

    let x = match preset.horizontal() {
        "left" => ax + ox,
        "right" => ax + aw - w - ox,
        _ => ax + (aw - w) / 2,
    };
    let y = match preset.vertical() {
        "top" => ay + oy,
        "bottom" => ay + ah - h - oy,
        _ => ay + (ah - h) / 2,
    };
    (x, y)
}

/// カーソル / アクティブウィンドウ追従モードのとき、モニターが変わったら移動する。
pub fn spawn_follow(app: AppHandle) {
    thread::Builder::new()
        .name("overlay-follow".into())
        .spawn(move || {
            let mut last: Option<String> = None;
            loop {
                thread::sleep(FOLLOW_INTERVAL);
                let state = app.state::<AppState>();
                let (target, position) = {
                    let c = state.config.lock().unwrap();
                    (c.monitor.clone(), c.position)
                };
                if !target.follows() || matches!(position, Position::Custom { .. }) || state.is_arranging() {
                    last = None;
                    continue;
                }
                let current = resolve_monitor(&app, &target).and_then(|m| m.name().cloned());
                if current.is_some() && current != last {
                    if last.is_some() {
                        let _ = apply_position(&app);
                    }
                    last = current;
                }
            }
        })
        .expect("spawn follow thread");
}
