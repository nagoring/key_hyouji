//! Windows: 低レベルキーボードフック（WH_KEYBOARD_LL）。

use super::HookError;
use crate::event::{
    InputEvent, Key, KeyEvent, Modifiers, MouseButton, MouseEvent, NamedKey, ScrollDirection, ScrollEvent,
};
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{sync_channel, Sender};
use std::sync::OnceLock;
use std::thread;

use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyState, GetKeyboardLayout, ToUnicodeEx, VIRTUAL_KEY, VK_BACK, VK_CAPITAL,
    VK_CONTROL, VK_CONVERT, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_INSERT, VK_KANA,
    VK_KANJI, VK_LCONTROL, VK_LEFT, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_NEXT, VK_NONCONVERT,
    VK_PRIOR, VK_RCONTROL, VK_RETURN, VK_RIGHT, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT, VK_SNAPSHOT,
    VK_SPACE, VK_TAB, VK_UP,
};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId,
    PostThreadMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, HC_ACTION, HHOOK,
    KBDLLHOOKSTRUCT, MSG, MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_APP, WM_KEYDOWN,
    WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_MOUSEHWHEEL, WM_MOUSEWHEEL, WM_RBUTTONDOWN, WM_SYSKEYDOWN,
};

static SINK: OnceLock<Sender<InputEvent>> = OnceLock::new();
static STARTED: AtomicBool = AtomicBool::new(false);
/// マウスフックを付けたいか（フックスレッド起動前に設定された場合も反映する）
static MOUSE_WANTED: AtomicBool = AtomicBool::new(false);
/// フックスレッドの ID（マウスフックの付け外しを依頼する）
static HOOK_THREAD: AtomicU32 = AtomicU32::new(0);
const WM_APP_MOUSE: u32 = WM_APP + 1;

thread_local! {
    static MOUSE_HOOK: Cell<Option<HHOOK>> = const { Cell::new(None) };
}

pub fn set_mouse_enabled(enabled: bool) {
    MOUSE_WANTED.store(enabled, Ordering::SeqCst);
    let tid = HOOK_THREAD.load(Ordering::SeqCst);
    if tid != 0 {
        unsafe {
            let _ = PostThreadMessageW(tid, WM_APP_MOUSE, WPARAM(0), LPARAM(0));
        }
    }
}

/// フックスレッド上で、MOUSE_WANTED に合わせてマウスフックを付け外しする。
unsafe fn sync_mouse_hook(hmod: Option<HINSTANCE>) {
    let want = MOUSE_WANTED.load(Ordering::SeqCst);
    MOUSE_HOOK.with(|cell| match (want, cell.get()) {
        (true, None) => match SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), hmod, 0) {
            Ok(h) => cell.set(Some(h)),
            Err(e) => log::warn!("failed to install mouse hook: {e}"),
        },
        (false, Some(h)) => {
            let _ = UnhookWindowsHookEx(h);
            cell.set(None);
        }
        _ => {}
    });
}

// OEM 系の日本語キー（windows クレートに定数が無い / 名前が環境で異なるため数値で持つ）
const VK_OEM_ATTN: u16 = 0xF0; // 英数（JIS の CapsLock 位置）
const VK_OEM_AUTO: u16 = 0xF3; // 半角/全角
const VK_OEM_ENLW: u16 = 0xF4; // 半角/全角
const VK_PROCESSKEY: u16 = 0xE5;
const VK_PACKET: u16 = 0xE7;

pub fn start(sink: Sender<InputEvent>) -> Result<(), HookError> {
    if STARTED.swap(true, Ordering::SeqCst) {
        return Err(HookError::Failed("hook already started".into()));
    }
    let _ = SINK.set(sink);

    let (tx, rx) = sync_channel::<Result<(), HookError>>(1);
    thread::Builder::new()
        .name("key-hook".into())
        .spawn(move || unsafe {
            let hmod: Option<HINSTANCE> = GetModuleHandleW(None).ok().map(Into::into);
            let hook = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), hmod, 0) {
                Ok(h) => {
                    let _ = tx.send(Ok(()));
                    h
                }
                Err(e) => {
                    let _ = tx.send(Err(HookError::Failed(e.to_string())));
                    return;
                }
            };
            HOOK_THREAD.store(GetCurrentThreadId(), Ordering::SeqCst);
            sync_mouse_hook(hmod);
            // LL フックはインストールしたスレッドのメッセージループで呼ばれる
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                if msg.hwnd.0.is_null() && msg.message == WM_APP_MOUSE {
                    sync_mouse_hook(hmod);
                    continue;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            MOUSE_WANTED.store(false, Ordering::SeqCst);
            sync_mouse_hook(hmod);
            let _ = UnhookWindowsHookEx(hook);
        })
        .map_err(|e| HookError::Failed(e.to_string()))?;

    let result = rx
        .recv()
        .unwrap_or_else(|_| Err(HookError::Failed("hook thread exited".into())));
    if result.is_err() {
        STARTED.store(false, Ordering::SeqCst);
    }
    result
}

pub fn has_permission() -> bool {
    true
}

pub fn request_permission() {}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let msg = wparam.0 as u32;
        if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
            let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            if let Some(ev) = translate(kb) {
                if let Some(sink) = SINK.get() {
                    let _ = sink.send(InputEvent::Key(ev));
                }
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

/// マウスの LL フック。移動のたびに呼ばれるので、ボタン押下・ホイール以外は即座に返す。
unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let msg = wparam.0 as u32;
        if msg == WM_MOUSEWHEEL || msg == WM_MOUSEHWHEEL {
            let ms = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            // mouseData の上位 16 ビットが回転量（符号付き）
            let delta = (ms.mouseData >> 16) as u16 as i16;
            let direction = match (msg == WM_MOUSEWHEEL, delta > 0) {
                (true, true) => Some(ScrollDirection::Up),
                (true, false) if delta < 0 => Some(ScrollDirection::Down),
                (false, true) => Some(ScrollDirection::Right),
                (false, false) if delta < 0 => Some(ScrollDirection::Left),
                _ => None,
            };
            if let (Some(direction), Some(sink)) = (direction, SINK.get()) {
                let _ = sink.send(InputEvent::Scroll(ScrollEvent { direction, modifiers: current_modifiers() }));
            }
            return CallNextHookEx(None, code, wparam, lparam);
        }
        let button = match msg {
            WM_LBUTTONDOWN => Some(MouseButton::Left),
            WM_RBUTTONDOWN => Some(MouseButton::Right),
            WM_MBUTTONDOWN => Some(MouseButton::Middle),
            _ => None,
        };
        if let Some(button) = button {
            let ms = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            let ev = MouseEvent { button, x: ms.pt.x as f64, y: ms.pt.y as f64, modifiers: current_modifiers() };
            if let Some(sink) = SINK.get() {
                let _ = sink.send(InputEvent::Mouse(ev));
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

fn pressed(vk: VIRTUAL_KEY) -> bool {
    unsafe { (GetAsyncKeyState(vk.0 as i32) as u16 & 0x8000) != 0 }
}

fn current_modifiers() -> Modifiers {
    Modifiers {
        ctrl: pressed(VK_CONTROL),
        alt: pressed(VK_MENU),
        shift: pressed(VK_SHIFT),
        meta: pressed(VK_LWIN) || pressed(VK_RWIN),
    }
}

unsafe fn translate(kb: &KBDLLHOOKSTRUCT) -> Option<KeyEvent> {
    let vk = kb.vkCode as u16;
    if vk == VK_PROCESSKEY || vk == VK_PACKET {
        return None;
    }
    let mut modifiers = current_modifiers();
    // 押した修飾キー自身は modifiers に含めない
    if let Some(n) = named_key(vk) {
        match n {
            NamedKey::Shift => modifiers.shift = false,
            NamedKey::Control => modifiers.ctrl = false,
            NamedKey::Alt => modifiers.alt = false,
            NamedKey::Meta => modifiers.meta = false,
            _ => {}
        }
        return Some(KeyEvent::new(Key::Named(n), None, modifiers));
    }

    let hkl = {
        let fg = GetForegroundWindow();
        let tid = GetWindowThreadProcessId(fg, None);
        GetKeyboardLayout(tid)
    };
    let scan = kb.scanCode;

    // 組み合わせ表示用の基本キー
    let base = match vk {
        0x41..=0x5A => Some(((vk as u8) as char).to_ascii_lowercase().to_string()),
        0x30..=0x39 => Some(((vk as u8) as char).to_string()),
        0x60..=0x69 => Some(((b'0' + (vk - 0x60) as u8) as char).to_string()),
        _ => to_unicode(vk, scan, &[0u8; 256], hkl),
    }?;
    if base.chars().any(char::is_control) {
        return None;
    }

    // 実際に入力される文字（Shift / CapsLock を反映）
    let text = if modifiers.has_command() {
        None
    } else {
        let mut state = [0u8; 256];
        if modifiers.shift {
            state[VK_SHIFT.0 as usize] = 0x80;
        }
        if GetKeyState(VK_CAPITAL.0 as i32) & 1 != 0 {
            state[VK_CAPITAL.0 as usize] = 0x01;
        }
        to_unicode(vk, scan, &state, hkl)
    };

    Some(KeyEvent { key: Key::Char(base), text, modifiers })
}

unsafe fn to_unicode(
    vk: u16,
    scan: u32,
    state: &[u8; 256],
    hkl: windows::Win32::UI::Input::KeyboardAndMouse::HKL,
) -> Option<String> {
    let mut buf = [0u16; 8];
    // wFlags bit 2 (0x4): キーボードの内部状態（デッドキー等）を変更しない
    let n = ToUnicodeEx(vk as u32, scan, state, &mut buf, 0x4, Some(hkl));
    if n > 0 {
        Some(String::from_utf16_lossy(&buf[..n as usize]))
    } else {
        None
    }
}

fn named_key(vk: u16) -> Option<NamedKey> {
    let v = VIRTUAL_KEY(vk);
    Some(match v {
        VK_RETURN => NamedKey::Enter,
        VK_TAB => NamedKey::Tab,
        VK_SPACE => NamedKey::Space,
        VK_BACK => NamedKey::Backspace,
        VK_DELETE => NamedKey::Delete,
        VK_ESCAPE => NamedKey::Escape,
        VK_LEFT => NamedKey::Left,
        VK_RIGHT => NamedKey::Right,
        VK_UP => NamedKey::Up,
        VK_DOWN => NamedKey::Down,
        VK_HOME => NamedKey::Home,
        VK_END => NamedKey::End,
        VK_PRIOR => NamedKey::PageUp,
        VK_NEXT => NamedKey::PageDown,
        VK_INSERT => NamedKey::Insert,
        VK_CAPITAL => NamedKey::CapsLock,
        VK_SNAPSHOT => NamedKey::PrintScreen,
        VK_SHIFT | VK_LSHIFT | VK_RSHIFT => NamedKey::Shift,
        VK_CONTROL | VK_LCONTROL | VK_RCONTROL => NamedKey::Control,
        VK_MENU | VK_LMENU | VK_RMENU => NamedKey::Alt,
        VK_LWIN | VK_RWIN => NamedKey::Meta,
        VK_CONVERT => NamedKey::Henkan,
        VK_NONCONVERT => NamedKey::Muhenkan,
        VK_KANA => NamedKey::Kana,
        VK_KANJI => NamedKey::HankakuZenkaku,
        _ => match vk {
            0x70..=0x87 => NamedKey::F((vk - 0x70 + 1) as u8),
            VK_OEM_ATTN => NamedKey::Eisu,
            VK_OEM_AUTO | VK_OEM_ENLW => NamedKey::HankakuZenkaku,
            _ => return None,
        },
    })
}
