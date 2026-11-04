//! システムトレイ / メニューバー常駐（C-01）。

use crate::state::{self, AppState, HookStatus};
use crate::windows;
use key_core::config::{Position, Preset};
use key_core::format::hotkey_label;
use key_core::{DisplayMode, Platform};
use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry};

const TRAY_ID: &str = "main";

pub struct TrayMenu {
    pause: CheckMenuItem<Wry>,
    mode_all: CheckMenuItem<Wry>,
    mode_shortcuts: CheckMenuItem<Wry>,
    presets: Vec<(Preset, CheckMenuItem<Wry>)>,
    arrange: CheckMenuItem<Wry>,
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let settings = MenuItem::with_id(app, "settings", "設定…", true, None::<&str>)?;
    let pause = CheckMenuItem::with_id(app, "pause", "一時停止", true, false, None::<&str>)?;

    let mode_all = CheckMenuItem::with_id(app, "mode:all", "すべてのキー", true, true, None::<&str>)?;
    let mode_shortcuts = CheckMenuItem::with_id(app, "mode:shortcuts", "ショートカットのみ", true, false, None::<&str>)?;
    let mode_menu = Submenu::with_items(app, "表示モード", true, &[&mode_all, &mode_shortcuts])?;

    let presets = Preset::ALL
        .into_iter()
        .map(|p| {
            CheckMenuItem::with_id(app, format!("pos:{}", p.id()), p.label(), true, false, None::<&str>)
                .map(|item| (p, item))
        })
        .collect::<tauri::Result<Vec<_>>>()?;
    let arrange = CheckMenuItem::with_id(app, "arrange", "配置モード（ドラッグで移動）", true, false, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let mut pos_items: Vec<&dyn IsMenuItem<Wry>> = presets.iter().map(|(_, i)| i as &dyn IsMenuItem<Wry>).collect();
    pos_items.push(&separator);
    pos_items.push(&arrange);
    let pos_menu = Submenu::with_items(app, "表示位置", true, &pos_items)?;

    let quit = MenuItem::with_id(app, "quit", "終了", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;

    #[cfg(target_os = "macos")]
    let privacy = MenuItem::with_id(app, "onboarding", "入力監視の許可について…", true, None::<&str>)?;

    let mut items: Vec<&dyn IsMenuItem<Wry>> = vec![&settings, &pause, &sep1, &mode_menu, &pos_menu];
    #[cfg(target_os = "macos")]
    items.push(&privacy);
    items.push(&sep2);
    items.push(&quit);
    let menu = Menu::with_items(app, &items)?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("key_hyouji")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| handle_menu(app, event.id().as_ref()));
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;

    app.manage(TrayMenu { pause, mode_all, mode_shortcuts, presets, arrange });
    sync(app);
    Ok(())
}

fn handle_menu(app: &AppHandle, id: &str) {
    match id {
        "pause" => state::toggle_paused(app),
        "mode:all" => state::set_display_mode(app, DisplayMode::All),
        "mode:shortcuts" => state::set_display_mode(app, DisplayMode::ShortcutsOnly),
        "arrange" => {
            let on = !app.state::<AppState>().is_arranging();
            state::set_arranging(app, on);
        }
        "settings" => windows::open_settings(app),
        "onboarding" => windows::open_onboarding(app),
        "quit" => app.exit(0),
        _ => {
            if let Some(p) = id.strip_prefix("pos:").and_then(Preset::from_id) {
                state::set_preset(app, p);
            }
        }
    }
}

/// メニューのチェック状態とツールチップを現在の状態に合わせる。
/// （クリックでチェックが自動反転するため、操作後は必ず呼ぶ）
pub fn sync(app: &AppHandle) {
    let Some(menu) = app.try_state::<TrayMenu>() else { return };
    let s = app.state::<AppState>();
    let (mode, position, hotkey) = {
        let c = s.config.lock().unwrap();
        (c.display_mode, c.position, c.pause_hotkey.clone())
    };
    let paused = s.is_paused();
    let _ = menu.pause.set_checked(paused);
    let _ = menu.pause.set_text(format!("一時停止（{}）", hotkey_label(&hotkey, Platform::current())));
    let _ = menu.mode_all.set_checked(mode == DisplayMode::All);
    let _ = menu.mode_shortcuts.set_checked(mode == DisplayMode::ShortcutsOnly);
    for (p, item) in &menu.presets {
        let _ = item.set_checked(position == Position::Preset { preset: *p });
    }
    let _ = menu.arrange.set_checked(s.is_arranging());

    let tooltip = match (paused, s.hook_status()) {
        (_, HookStatus::NeedPermission) => "key_hyouji（入力監視の許可が必要）".to_owned(),
        (_, HookStatus::Error { .. }) => "key_hyouji（エラー）".to_owned(),
        (true, _) => "key_hyouji（一時停止中）".to_owned(),
        _ => "key_hyouji".to_owned(),
    };
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(tooltip));
    }
}
