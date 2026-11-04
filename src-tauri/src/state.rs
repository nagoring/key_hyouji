//! アプリ全体の状態と、設定変更の適用。
//!
//! 設定の変更はトレイ・設定画面・配置モードのどこからでも [`apply_config`] を通す。
//! 保存 → 集約ロジック / オーバーレイ / ホットキー / トレイへの反映を一括で行う。

use crate::i18n::{self, Strings};
use crate::{overlay, tray, OVERLAY};
use key_core::config::{Config, ImeMode, Language, Position, Preset};
use key_core::foreground::{self, ForegroundApp};
use key_core::{Aggregator, DisplayMode};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

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

/// オーバーレイの描画に必要な設定。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub display_duration_ms: u64,
    pub fade_duration_ms: u64,
    pub max_items: u32,
    pub theme: key_core::config::Theme,
    pub chroma_key: key_core::config::ChromaKey,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayState {
    pub layout: Layout,
    pub settings: Settings,
    pub paused: bool,
    pub hook: HookStatus,
}

/// フォアグラウンドアプリの問い合わせ結果を短時間キャッシュする（キー入力ごとの負荷を抑える）
const FOREGROUND_CACHE: Duration = Duration::from_millis(300);
/// IME 状態の問い合わせ結果のキャッシュ時間
const IME_CACHE: Duration = Duration::from_millis(200);

pub struct AppState {
    config_dir: PathBuf,
    pub config: Mutex<Config>,
    pub aggregator: Mutex<Aggregator>,
    paused: AtomicBool,
    arranging: AtomicBool,
    hook: Mutex<HookStatus>,
    foreground: Mutex<Option<(Instant, Option<ForegroundApp>)>>,
    ime: Mutex<Option<(Instant, bool)>>,
    os_locale: Option<String>,
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
            foreground: Mutex::new(None),
            ime: Mutex::new(None),
            os_locale: sys_locale::get_locale(),
        }
    }

    /// 実際に使う言語（`Auto` を OS の言語で解決したもの）。
    pub fn language(&self) -> Language {
        let lang = self.config.lock().unwrap().language;
        lang.resolve(self.os_locale.as_deref())
    }

    pub fn strings(&self) -> &'static Strings {
        i18n::strings(self.language())
    }

    /// 集約ロジックの設定（クリックの表示名は言語に合わせる）。
    pub fn aggregator_config(&self, config: &Config) -> key_core::AggregatorConfig {
        let lang = config.language.resolve(self.os_locale.as_deref());
        let tr = i18n::strings(lang);
        key_core::AggregatorConfig {
            click_labels: tr.clicks.map(str::to_owned),
            scroll_label: tr.scroll.to_owned(),
            ..config.aggregator_config()
        }
    }

    pub fn config(&self) -> Config {
        self.config.lock().unwrap().clone()
    }

    pub fn config_path(&self) -> PathBuf {
        Config::path(&self.config_dir)
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
            theme: c.theme.clone(),
            chroma_key: c.chroma_key.clone(),
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

    /// 最前面のアプリが除外対象か（P-03）。自分自身（設定画面など）は対象外。
    pub fn foreground_excluded(&self) -> bool {
        let config = self.config.lock().unwrap();
        if config.excluded_apps.is_empty() {
            return false;
        }
        let mut cache = self.foreground.lock().unwrap();
        let fresh = matches!(&*cache, Some((at, _)) if at.elapsed() < FOREGROUND_CACHE);
        if !fresh {
            *cache = Some((Instant::now(), foreground::foreground_app()));
        }
        match &*cache {
            Some((_, Some(app))) => !app.is_self && config.is_excluded(&app.name),
            _ => false,
        }
    }
}

impl AppState {
    /// IME で文字を変換する入力モードか（F-11）。短時間キャッシュする。
    pub fn ime_active(&self, app: &AppHandle) -> bool {
        let mut cache = self.ime.lock().unwrap();
        if let Some((at, v)) = *cache {
            if at.elapsed() < IME_CACHE {
                return v;
            }
        }
        let v = query_ime(app);
        *cache = Some((Instant::now(), v));
        v
    }

    pub fn ime_mode(&self) -> ImeMode {
        self.config.lock().unwrap().ime_mode
    }
}

#[cfg(target_os = "macos")]
fn query_ime(app: &AppHandle) -> bool {
    // Text Input Sources API はメインスレッドで呼ぶ
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    if app
        .run_on_main_thread(move || {
            let _ = tx.send(key_core::ime::ime_active());
        })
        .is_err()
    {
        return false;
    }
    rx.recv_timeout(Duration::from_millis(150)).unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
fn query_ime(_app: &AppHandle) -> bool {
    key_core::ime::ime_active()
}

fn state(app: &AppHandle) -> tauri::State<'_, AppState> {
    app.state::<AppState>()
}

/// 設定文字列からホットキーを作る。
pub fn parse_hotkey(spec: &str) -> Result<Shortcut, String> {
    spec.parse::<Shortcut>().map_err(|e| format!("{spec}: {e}"))
}

/// 起動時のホットキー登録。失敗してもアプリは起動を続ける。
pub fn register_initial_hotkey(app: &AppHandle) {
    let spec = state(app).config.lock().unwrap().pause_hotkey.clone();
    match parse_hotkey(&spec).and_then(|s| app.global_shortcut().register(s).map_err(|e| e.to_string())) {
        Ok(()) => {}
        Err(e) => log::warn!("failed to register pause hotkey: {e}"),
    }
}

fn swap_hotkey(app: &AppHandle, old: &str, new: &str) -> Result<(), String> {
    let tr = state(app).strings();
    let new_sc = parse_hotkey(new).map_err(|e| tr.with_paren(tr.err_hotkey_parse, &e))?;
    let gs = app.global_shortcut();
    if let Ok(old_sc) = parse_hotkey(old) {
        let _ = gs.unregister(old_sc);
    }
    if let Err(e) = gs.register(new_sc) {
        // 元に戻す
        if let Ok(old_sc) = parse_hotkey(old) {
            let _ = gs.register(old_sc);
        }
        return Err(format!("{}: {e}", tr.err_hotkey_register));
    }
    Ok(())
}

/// 設定を検証・保存し、すべての表示に反映する。
pub fn apply_config(app: &AppHandle, new: Config) -> Result<Config, String> {
    let s = state(app);
    let new = new.sanitized();
    let old = s.config();
    if new == old {
        return Ok(new);
    }
    if new.auto_start != old.auto_start {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        key_core::autostart::set_enabled(new.auto_start, &exe)
            .map_err(|e| format!("{}: {e}", s.strings().err_autostart))?;
    }
    if new.pause_hotkey != old.pause_hotkey {
        if let Err(e) = swap_hotkey(app, &old.pause_hotkey, &new.pause_hotkey) {
            // 自動起動だけ変わった状態にならないよう戻す
            if new.auto_start != old.auto_start {
                if let Ok(exe) = std::env::current_exe() {
                    let _ = key_core::autostart::set_enabled(old.auto_start, &exe);
                }
            }
            return Err(e);
        }
    }

    {
        let mut c = s.config.lock().unwrap();
        *c = new.clone();
        c.save(&s.config_dir);
    }
    s.aggregator.lock().unwrap().set_config(s.aggregator_config(&new));
    if new.mouse != old.mouse {
        key_core::hook::set_mouse_enabled(new.mouse.any_enabled());
    }
    *s.foreground.lock().unwrap() = None;

    let geometry_changed = new.position != old.position
        || new.monitor != old.monitor
        || new.offset_x != old.offset_x
        || new.offset_y != old.offset_y
        || new.max_items != old.max_items
        || new.theme.font_size != old.theme.font_size;
    if geometry_changed && !s.is_arranging() {
        if let Err(e) = overlay::apply_position(app) {
            log::error!("failed to move overlay: {e}");
        }
    }
    let _ = app.emit_to(OVERLAY, "settings", s.settings());
    let _ = app.emit_to(OVERLAY, "layout", s.layout());
    if !new.show_secure_indicator {
        let _ = app.emit_to(OVERLAY, "secure-input", false);
    }
    let _ = app.emit("config-changed", &new);
    if new.language != old.language {
        let _ = app.emit("language-changed", s.language().code());
        tray::relabel(app);
        crate::windows::retitle(app);
    }
    tray::sync(app);
    Ok(new)
}

fn update(app: &AppHandle, f: impl FnOnce(&mut Config)) {
    let mut c = state(app).config();
    f(&mut c);
    if let Err(e) = apply_config(app, c) {
        log::error!("{e}");
    }
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

pub fn toggle_clicks(app: &AppHandle) {
    update(app, |c| c.mouse.show_clicks = !c.mouse.show_clicks);
}

pub fn set_display_mode(app: &AppHandle, mode: DisplayMode) {
    update(app, |c| c.display_mode = mode);
}

pub fn set_preset(app: &AppHandle, preset: Preset) {
    if state(app).is_arranging() {
        stop_arranging(app, false);
    }
    update(app, |c| c.position = Position::Preset { preset });
    // 同じプリセットを選び直した場合も位置を戻す
    let _ = overlay::apply_position(app);
}

/// 配置モード（W-04）。オン: ドラッグで移動可能。オフ: 現在位置を保存してクリック透過に戻す。
pub fn set_arranging(app: &AppHandle, on: bool) {
    if on {
        let s = state(app);
        let Some(window) = app.get_webview_window(OVERLAY) else { return };
        s.arranging.store(true, Ordering::SeqCst);
        let _ = window.set_ignore_cursor_events(false);
        let _ = window.set_focus();
        let _ = app.emit_to(OVERLAY, "layout", s.layout());
        let _ = app.emit("arranging", true);
        tray::sync(app);
    } else {
        stop_arranging(app, true);
    }
}

fn stop_arranging(app: &AppHandle, save_position: bool) {
    let s = state(app);
    if !s.arranging.swap(false, Ordering::SeqCst) {
        return;
    }
    let Some(window) = app.get_webview_window(OVERLAY) else { return };
    let _ = window.set_ignore_cursor_events(true);
    if save_position {
        if let Ok(p) = window.outer_position() {
            update(app, |c| {
                let align = c.position.align();
                c.position = Position::Custom { x: p.x, y: p.y, align };
            });
        }
    }
    let _ = app.emit_to(OVERLAY, "layout", s.layout());
    let _ = app.emit("arranging", false);
    tray::sync(app);
}

pub fn set_hook_status(app: &AppHandle, status: HookStatus) {
    *state(app).hook.lock().unwrap() = status.clone();
    let _ = app.emit("hook-status", status);
    tray::sync(app);
}
