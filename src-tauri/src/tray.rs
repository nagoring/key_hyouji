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
    settings: MenuItem<Wry>,
    pause: CheckMenuItem<Wry>,
    mode_menu: Submenu<Wry>,
    mode_all: CheckMenuItem<Wry>,
    mode_shortcuts: CheckMenuItem<Wry>,
    clicks: CheckMenuItem<Wry>,
    pos_menu: Submenu<Wry>,
    presets: Vec<(Preset, CheckMenuItem<Wry>)>,
    arrange: CheckMenuItem<Wry>,
    onboarding: Option<MenuItem<Wry>>,
    quit: MenuItem<Wry>,
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let tr = app.state::<AppState>().strings();
    let settings = MenuItem::with_id(app, "settings", tr.settings, true, None::<&str>)?;
    let pause = CheckMenuItem::with_id(app, "pause", tr.pause, true, false, None::<&str>)?;

    let mode_all = CheckMenuItem::with_id(app, "mode:all", tr.mode_all, true, true, None::<&str>)?;
    let mode_shortcuts = CheckMenuItem::with_id(app, "mode:shortcuts", tr.mode_shortcuts, true, false, None::<&str>)?;
    let mode_menu = Submenu::with_items(app, tr.display_mode, true, &[&mode_all, &mode_shortcuts])?;
    let clicks = CheckMenuItem::with_id(app, "clicks", tr.show_clicks, true, false, None::<&str>)?;

    let presets = Preset::ALL
        .into_iter()
        .map(|p| {
            CheckMenuItem::with_id(app, format!("pos:{}", p.id()), tr.preset(p), true, false, None::<&str>)
                .map(|item| (p, item))
        })
        .collect::<tauri::Result<Vec<_>>>()?;
    let arrange = CheckMenuItem::with_id(app, "arrange", tr.arrange, true, false, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let mut pos_items: Vec<&dyn IsMenuItem<Wry>> = presets.iter().map(|(_, i)| i as &dyn IsMenuItem<Wry>).collect();
    pos_items.push(&separator);
    pos_items.push(&arrange);
    let pos_menu = Submenu::with_items(app, tr.position, true, &pos_items)?;

    let quit = MenuItem::with_id(app, "quit", tr.quit, true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;

    // macOS のみ: 入力監視の案内
    let onboarding = if cfg!(target_os = "macos") {
        Some(MenuItem::with_id(app, "onboarding", tr.onboarding, true, None::<&str>)?)
    } else {
        None
    };

    let mut items: Vec<&dyn IsMenuItem<Wry>> = vec![&settings, &pause, &sep1, &mode_menu, &clicks, &pos_menu];
    if let Some(o) = &onboarding {
        items.push(o);
    }
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

    app.manage(TrayMenu {
        settings,
        pause,
        mode_menu,
        mode_all,
        mode_shortcuts,
        clicks,
        pos_menu,
        presets,
        arrange,
        onboarding,
        quit,
    });
    sync(app);
    Ok(())
}

fn handle_menu(app: &AppHandle, id: &str) {
    match id {
        "pause" => state::toggle_paused(app),
        "clicks" => state::toggle_clicks(app),
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

/// 言語が変わったときにメニューの文言を差し替える（C-06）。
pub fn relabel(app: &AppHandle) {
    let Some(menu) = app.try_state::<TrayMenu>() else { return };
    let tr = app.state::<AppState>().strings();
    let _ = menu.settings.set_text(tr.settings);
    let _ = menu.mode_menu.set_text(tr.display_mode);
    let _ = menu.mode_all.set_text(tr.mode_all);
    let _ = menu.mode_shortcuts.set_text(tr.mode_shortcuts);
    let _ = menu.clicks.set_text(tr.show_clicks);
    let _ = menu.pos_menu.set_text(tr.position);
    for (p, item) in &menu.presets {
        let _ = item.set_text(tr.preset(*p));
    }
    let _ = menu.arrange.set_text(tr.arrange);
    if let Some(o) = &menu.onboarding {
        let _ = o.set_text(tr.onboarding);
    }
    let _ = menu.quit.set_text(tr.quit);
}

/// メニューのチェック状態とツールチップを現在の状態に合わせる。
/// （クリックでチェックが自動反転するため、操作後は必ず呼ぶ）
pub fn sync(app: &AppHandle) {
    let Some(menu) = app.try_state::<TrayMenu>() else { return };
    let s = app.state::<AppState>();
    let tr = s.strings();
    let (mode, position, hotkey, show_clicks) = {
        let c = s.config.lock().unwrap();
        (c.display_mode, c.position, c.pause_hotkey.clone(), c.mouse.show_clicks)
    };
    let paused = s.is_paused();
    let _ = menu.pause.set_checked(paused);
    let _ = menu.pause.set_text(tr.with_paren(tr.pause, &hotkey_label(&hotkey, Platform::current())));
    let _ = menu.mode_all.set_checked(mode == DisplayMode::All);
    let _ = menu.mode_shortcuts.set_checked(mode == DisplayMode::ShortcutsOnly);
    let _ = menu.clicks.set_checked(show_clicks);
    for (p, item) in &menu.presets {
        let _ = item.set_checked(position == Position::Preset { preset: *p });
    }
    let _ = menu.arrange.set_checked(s.is_arranging());

    let status = match (paused, s.hook_status()) {
        (_, HookStatus::NeedPermission) => Some(tr.tooltip_need_permission),
        (_, HookStatus::Error { .. }) => Some(tr.tooltip_error),
        (true, _) => Some(tr.tooltip_paused),
        _ => None,
    };
    let tooltip = match status {
        Some(st) => tr.with_paren("key_hyouji", st),
        None => "key_hyouji".to_owned(),
    };
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(tooltip));
    }
}
