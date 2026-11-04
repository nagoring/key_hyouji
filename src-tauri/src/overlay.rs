//! オーバーレイウィンドウの大きさと位置（W-04）。

use crate::state::AppState;
use crate::OVERLAY;
use key_core::config::{Position, Preset};
use tauri::{AppHandle, Manager, Monitor, PhysicalPosition, PhysicalSize};

/// オーバーレイの大きさ（論理ピクセル）。最大 3 件程度のキー表示が収まる大きさ。
const WIDTH: f64 = 1000.0;
const HEIGHT: f64 = 300.0;

pub fn apply_position(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(OVERLAY) else { return Ok(()) };
    let (position, ox, oy) = {
        let state = app.state::<AppState>();
        let c = state.config.lock().unwrap();
        (c.position, c.offset_x, c.offset_y)
    };
    let monitors = window.available_monitors()?;
    let primary = window.primary_monitor()?.or_else(|| monitors.first().cloned());
    let Some(primary) = primary else { return Ok(()) };

    let (monitor, x, y) = match position {
        Position::Custom { x, y, .. } => match monitor_containing(&monitors, x, y, &primary) {
            Some(m) => (m.clone(), x, y),
            // 保存位置のモニターが無くなった場合は既定位置へ
            None => {
                let (x, y) = preset_origin(&primary, Preset::BottomCenter, ox, oy);
                (primary.clone(), x, y)
            }
        },
        Position::Preset { preset } => {
            let (x, y) = preset_origin(&primary, preset, ox, oy);
            (primary.clone(), x, y)
        }
    };
    window.set_size(physical_size(&monitor))?;
    window.set_position(PhysicalPosition::new(x, y))?;
    Ok(())
}

fn physical_size(m: &Monitor) -> PhysicalSize<u32> {
    let s = m.scale_factor();
    PhysicalSize::new((WIDTH * s).round() as u32, (HEIGHT * s).round() as u32)
}

/// ウィンドウ左上が (x, y) のとき、ウィンドウ中心を含むモニター。
fn monitor_containing<'a>(monitors: &'a [Monitor], x: i32, y: i32, primary: &Monitor) -> Option<&'a Monitor> {
    let size = physical_size(primary);
    let cx = x + size.width as i32 / 2;
    let cy = y + size.height as i32 / 2;
    monitors.iter().find(|m| {
        let p = m.position();
        let s = m.size();
        cx >= p.x && cx < p.x + s.width as i32 && cy >= p.y && cy < p.y + s.height as i32
    })
}

/// プリセット位置でのウィンドウ左上座標（タスクバー / Dock を除いた作業領域基準）。
fn preset_origin(m: &Monitor, preset: Preset, offset_x: f64, offset_y: f64) -> (i32, i32) {
    let s = m.scale_factor();
    let area = m.work_area();
    let size = physical_size(m);
    let (ax, ay) = (area.position.x, area.position.y);
    let (aw, ah) = (area.size.width as i32, area.size.height as i32);
    let (w, h) = (size.width as i32, size.height as i32);
    let ox = (offset_x * s).round() as i32;
    let oy = (offset_y * s).round() as i32;

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
