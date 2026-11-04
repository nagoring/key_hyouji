//! macOS: CGEventTap（listen-only）。「入力監視」権限が必要。

use super::HookError;
use crate::event::{Key, KeyEvent, Modifiers, NamedKey};
use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::mpsc::{sync_channel, Sender};
use std::sync::OnceLock;
use std::thread;

type CFMachPortRef = *mut c_void;
type CFRunLoopSourceRef = *mut c_void;
type CFRunLoopRef = *mut c_void;
type CGEventRef = *mut c_void;
type CGEventTapProxy = *mut c_void;
type CGEventTapCallBack =
    unsafe extern "C" fn(CGEventTapProxy, u32, CGEventRef, *mut c_void) -> CGEventRef;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: CGEventTapCallBack,
        user_info: *mut c_void,
    ) -> CFMachPortRef;
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    fn CGEventGetFlags(event: CGEventRef) -> u64;
    fn CGEventKeyboardGetUnicodeString(
        event: CGEventRef,
        max_len: usize,
        actual_len: *mut usize,
        buf: *mut u16,
    );
    fn CGPreflightListenEventAccess() -> bool;
    fn CGRequestListenEventAccess() -> bool;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    static kCFRunLoopCommonModes: *const c_void;
    fn CFMachPortCreateRunLoopSource(
        allocator: *const c_void,
        port: CFMachPortRef,
        order: isize,
    ) -> CFRunLoopSourceRef;
    fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: *const c_void);
    fn CFRunLoopRun();
}

const K_CG_SESSION_EVENT_TAP: u32 = 1;
const K_CG_HEAD_INSERT_EVENT_TAP: u32 = 0;
const K_CG_EVENT_TAP_OPTION_LISTEN_ONLY: u32 = 1;

const K_CG_EVENT_KEY_DOWN: u32 = 10;
const K_CG_EVENT_FLAGS_CHANGED: u32 = 12;
const K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFF_FFFE;
const K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT: u32 = 0xFFFF_FFFF;

const K_CG_KEYBOARD_EVENT_KEYCODE: u32 = 9;

const FLAG_SHIFT: u64 = 1 << 17;
const FLAG_CONTROL: u64 = 1 << 18;
const FLAG_ALTERNATE: u64 = 1 << 19;
const FLAG_COMMAND: u64 = 1 << 20;

static SINK: OnceLock<Sender<KeyEvent>> = OnceLock::new();
static TAP: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static STARTED: AtomicBool = AtomicBool::new(false);

pub fn has_permission() -> bool {
    unsafe { CGPreflightListenEventAccess() }
}

pub fn request_permission() {
    unsafe {
        CGRequestListenEventAccess();
    }
}

pub fn start(sink: Sender<KeyEvent>) -> Result<(), HookError> {
    if STARTED.swap(true, Ordering::SeqCst) {
        return Err(HookError::Failed("hook already started".into()));
    }
    let _ = SINK.set(sink);

    let (tx, rx) = sync_channel::<Result<(), HookError>>(1);
    thread::Builder::new()
        .name("key-hook".into())
        .spawn(move || unsafe {
            let mask = (1u64 << K_CG_EVENT_KEY_DOWN) | (1u64 << K_CG_EVENT_FLAGS_CHANGED);
            let tap = CGEventTapCreate(
                K_CG_SESSION_EVENT_TAP,
                K_CG_HEAD_INSERT_EVENT_TAP,
                K_CG_EVENT_TAP_OPTION_LISTEN_ONLY,
                mask,
                tap_callback,
                ptr::null_mut(),
            );
            if tap.is_null() {
                let err = if CGPreflightListenEventAccess() {
                    HookError::Failed("CGEventTapCreate returned null".into())
                } else {
                    HookError::PermissionDenied
                };
                let _ = tx.send(Err(err));
                return;
            }
            TAP.store(tap, Ordering::SeqCst);
            let source = CFMachPortCreateRunLoopSource(ptr::null(), tap, 0);
            CFRunLoopAddSource(CFRunLoopGetCurrent(), source, kCFRunLoopCommonModes);
            CGEventTapEnable(tap, true);
            let _ = tx.send(Ok(()));
            CFRunLoopRun();
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

unsafe extern "C" fn tap_callback(
    _proxy: CGEventTapProxy,
    ty: u32,
    event: CGEventRef,
    _user_info: *mut c_void,
) -> CGEventRef {
    match ty {
        K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT | K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT => {
            // 応答が遅れると OS にタップを無効化されるので再有効化する
            let tap = TAP.load(Ordering::SeqCst);
            if !tap.is_null() {
                CGEventTapEnable(tap, true);
            }
        }
        K_CG_EVENT_KEY_DOWN | K_CG_EVENT_FLAGS_CHANGED => {
            if let Some(ev) = translate(ty, event) {
                if let Some(sink) = SINK.get() {
                    let _ = sink.send(ev);
                }
            }
        }
        _ => {}
    }
    event
}

fn modifiers_from_flags(flags: u64) -> Modifiers {
    Modifiers {
        ctrl: flags & FLAG_CONTROL != 0,
        alt: flags & FLAG_ALTERNATE != 0,
        shift: flags & FLAG_SHIFT != 0,
        meta: flags & FLAG_COMMAND != 0,
    }
}

unsafe fn event_string(event: CGEventRef) -> Option<String> {
    let mut buf = [0u16; 8];
    let mut len: usize = 0;
    CGEventKeyboardGetUnicodeString(event, buf.len(), &mut len, buf.as_mut_ptr());
    if len == 0 {
        return None;
    }
    let s = String::from_utf16_lossy(&buf[..len.min(buf.len())]);
    if s.chars().any(char::is_control) {
        None
    } else {
        Some(s)
    }
}

unsafe fn translate(ty: u32, event: CGEventRef) -> Option<KeyEvent> {
    let keycode = CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE) as u16;
    let flags = CGEventGetFlags(event);
    let mut modifiers = modifiers_from_flags(flags);

    if ty == K_CG_EVENT_FLAGS_CHANGED {
        // 修飾キーの押下 / 解放。押下時のみ通知する
        let (named, flag) = match keycode {
            56 | 60 => (NamedKey::Shift, FLAG_SHIFT),
            59 | 62 => (NamedKey::Control, FLAG_CONTROL),
            58 | 61 => (NamedKey::Alt, FLAG_ALTERNATE),
            55 | 54 => (NamedKey::Meta, FLAG_COMMAND),
            57 => return Some(KeyEvent::new(Key::Named(NamedKey::CapsLock), None, modifiers)),
            _ => return None,
        };
        if flags & flag == 0 {
            return None;
        }
        match named {
            NamedKey::Shift => modifiers.shift = false,
            NamedKey::Control => modifiers.ctrl = false,
            NamedKey::Alt => modifiers.alt = false,
            NamedKey::Meta => modifiers.meta = false,
            _ => {}
        }
        return Some(KeyEvent::new(Key::Named(named), None, modifiers));
    }

    if let Some(n) = named_key(keycode) {
        return Some(KeyEvent::new(Key::Named(n), None, modifiers));
    }

    let typed = event_string(event);
    // 組み合わせ表示用の基本キー。⌃/⌥ が無ければ OS が配列を反映した文字を使う
    let base = match (&typed, modifiers.ctrl || modifiers.alt) {
        (Some(s), false) if s.chars().count() == 1 => Some(s.to_lowercase()),
        _ => ansi_char(keycode).map(str::to_owned).or_else(|| typed.clone()),
    };
    let key = match base {
        Some(b) => Key::Char(b),
        None => Key::Unknown(format!("key{keycode}")),
    };
    let text = if modifiers.has_command() { None } else { typed };
    Some(KeyEvent { key, text, modifiers })
}

fn named_key(keycode: u16) -> Option<NamedKey> {
    Some(match keycode {
        36 | 76 => NamedKey::Enter,
        48 => NamedKey::Tab,
        49 => NamedKey::Space,
        51 => NamedKey::Backspace,
        117 => NamedKey::Delete,
        53 => NamedKey::Escape,
        123 => NamedKey::Left,
        124 => NamedKey::Right,
        125 => NamedKey::Down,
        126 => NamedKey::Up,
        115 => NamedKey::Home,
        119 => NamedKey::End,
        116 => NamedKey::PageUp,
        121 => NamedKey::PageDown,
        114 => NamedKey::Insert,
        122 => NamedKey::F(1),
        120 => NamedKey::F(2),
        99 => NamedKey::F(3),
        118 => NamedKey::F(4),
        96 => NamedKey::F(5),
        97 => NamedKey::F(6),
        98 => NamedKey::F(7),
        100 => NamedKey::F(8),
        101 => NamedKey::F(9),
        109 => NamedKey::F(10),
        103 => NamedKey::F(11),
        111 => NamedKey::F(12),
        105 => NamedKey::F(13),
        107 => NamedKey::F(14),
        113 => NamedKey::F(15),
        106 => NamedKey::F(16),
        64 => NamedKey::F(17),
        79 => NamedKey::F(18),
        80 => NamedKey::F(19),
        90 => NamedKey::F(20),
        102 => NamedKey::Eisu,
        104 => NamedKey::Kana,
        _ => return None,
    })
}

/// US 配列での位置に対応する文字（フォールバック用）。
fn ansi_char(keycode: u16) -> Option<&'static str> {
    Some(match keycode {
        0 => "a", 1 => "s", 2 => "d", 3 => "f", 4 => "h", 5 => "g", 6 => "z", 7 => "x",
        8 => "c", 9 => "v", 11 => "b", 12 => "q", 13 => "w", 14 => "e", 15 => "r",
        16 => "y", 17 => "t", 18 => "1", 19 => "2", 20 => "3", 21 => "4", 22 => "6",
        23 => "5", 24 => "=", 25 => "9", 26 => "7", 27 => "-", 28 => "8", 29 => "0",
        30 => "]", 31 => "o", 32 => "u", 33 => "[", 34 => "i", 35 => "p", 37 => "l",
        38 => "j", 39 => "'", 40 => "k", 41 => ";", 42 => "\\", 43 => ",", 44 => "/",
        45 => "n", 46 => "m", 47 => ".", 50 => "`", 93 => "¥", 94 => "_",
        65 => ".", 67 => "*", 69 => "+", 75 => "/", 78 => "-", 81 => "=",
        82 => "0", 83 => "1", 84 => "2", 85 => "3", 86 => "4", 87 => "5",
        88 => "6", 89 => "7", 91 => "8", 92 => "9",
        _ => return None,
    })
}
