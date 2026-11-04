//! クリック位置に表示するリング（F-10）。
//!
//! 小さな透過・クリック透過・最前面のウィンドウを起動時に 1 つ作っておき、
//! クリックのたびにその位置へ移動してアニメーションさせる。

use crate::state::AppState;
use key_core::{MouseButton, MouseEvent};
use serde::Serialize;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

pub const RING: &str = "ring";
/// リングウィンドウの大きさ（論理ピクセル）
const SIZE: f64 = 96.0;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RingPayload {
    button: MouseButton,
    color: String,
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let window = WebviewWindowBuilder::new(app, RING, WebviewUrl::App("ring.html".into()))
        .title("key_hyouji ring")
        .inner_size(SIZE, SIZE)
        .position(-10000.0, -10000.0)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible_on_all_workspaces(true)
        .build()?;
    window.set_ignore_cursor_events(true)?;
    Ok(())
}

pub fn show(app: &AppHandle, ev: &MouseEvent) {
    let Some(window) = app.get_webview_window(RING) else { return };
    let color = app.state::<AppState>().config().mouse.ring_color;

    if cfg!(target_os = "macos") {
        // macOS のクリック位置はポイント（論理座標）
        let _ = window.set_position(tauri::LogicalPosition::new(ev.x - SIZE / 2.0, ev.y - SIZE / 2.0));
    } else {
        // Windows は物理ピクセル。クリックしたモニターの拡大率で大きさを合わせる
        let scale = app
            .monitor_from_point(ev.x, ev.y)
            .ok()
            .flatten()
            .map(|m| m.scale_factor())
            .unwrap_or(1.0);
        let px = (SIZE * scale).round();
        let _ = window.set_position(PhysicalPosition::new((ev.x - px / 2.0) as i32, (ev.y - px / 2.0) as i32));
        let _ = window.set_size(PhysicalSize::new(px as u32, px as u32));
    }
    // 移動が反映されてからアニメーションを始める
    thread::sleep(Duration::from_millis(16));
    let _ = app.emit_to(RING, "ring", RingPayload { button: ev.button, color });
}
