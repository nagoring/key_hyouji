//! CGWindowListCopyWindowInfo で最前面（レイヤー 0）のウィンドウを調べる。
//! アプリ名・PID・位置は「画面収録」権限なしで取得できる。

use super::{ForegroundApp, Rect};
use std::ffi::{c_char, c_void, CStr};

type CFArrayRef = *const c_void;
type CFDictionaryRef = *const c_void;
type CFStringRef = *const c_void;
type CFNumberRef = *const c_void;

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct CGSize {
    width: f64,
    height: f64,
}
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    static kCGWindowLayer: CFStringRef;
    static kCGWindowOwnerName: CFStringRef;
    static kCGWindowOwnerPID: CFStringRef;
    static kCGWindowBounds: CFStringRef;
    fn CGWindowListCopyWindowInfo(option: u32, relative_to: u32) -> CFArrayRef;
    fn CGRectMakeWithDictionaryRepresentation(dict: CFDictionaryRef, rect: *mut CGRect) -> bool;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFArrayGetCount(array: CFArrayRef) -> isize;
    fn CFArrayGetValueAtIndex(array: CFArrayRef, idx: isize) -> *const c_void;
    fn CFDictionaryGetValue(dict: CFDictionaryRef, key: *const c_void) -> *const c_void;
    fn CFNumberGetValue(number: CFNumberRef, the_type: isize, value: *mut c_void) -> bool;
    fn CFStringGetCString(s: CFStringRef, buf: *mut c_char, size: isize, encoding: u32) -> bool;
    fn CFRelease(cf: *const c_void);
}

const K_CG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY: u32 = 1 << 0;
const K_CG_WINDOW_LIST_EXCLUDE_DESKTOP_ELEMENTS: u32 = 1 << 4;
const K_CF_NUMBER_SINT32_TYPE: isize = 3;
const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;

struct WindowInfo {
    layer: i32,
    pid: i32,
    owner: String,
    bounds: Option<Rect>,
}

unsafe fn number(dict: CFDictionaryRef, key: CFStringRef) -> Option<i32> {
    let v = CFDictionaryGetValue(dict, key);
    if v.is_null() {
        return None;
    }
    let mut out: i32 = 0;
    CFNumberGetValue(v, K_CF_NUMBER_SINT32_TYPE, &mut out as *mut i32 as *mut c_void).then_some(out)
}

unsafe fn string(dict: CFDictionaryRef, key: CFStringRef) -> Option<String> {
    let v = CFDictionaryGetValue(dict, key);
    if v.is_null() {
        return None;
    }
    let mut buf = [0 as c_char; 512];
    if !CFStringGetCString(v, buf.as_mut_ptr(), buf.len() as isize, K_CF_STRING_ENCODING_UTF8) {
        return None;
    }
    Some(CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned())
}

/// 画面上のウィンドウ一覧（前面から背面の順）。
fn windows() -> Vec<WindowInfo> {
    let mut out = Vec::new();
    unsafe {
        let list = CGWindowListCopyWindowInfo(
            K_CG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY | K_CG_WINDOW_LIST_EXCLUDE_DESKTOP_ELEMENTS,
            0,
        );
        if list.is_null() {
            return out;
        }
        for i in 0..CFArrayGetCount(list) {
            let dict = CFArrayGetValueAtIndex(list, i);
            if dict.is_null() {
                continue;
            }
            let bounds_dict = CFDictionaryGetValue(dict, kCGWindowBounds);
            let mut r = CGRect::default();
            let bounds = (!bounds_dict.is_null() && CGRectMakeWithDictionaryRepresentation(bounds_dict, &mut r))
                .then_some(Rect { x: r.origin.x, y: r.origin.y, width: r.size.width, height: r.size.height });
            out.push(WindowInfo {
                layer: number(dict, kCGWindowLayer).unwrap_or(-1),
                pid: number(dict, kCGWindowOwnerPID).unwrap_or(-1),
                owner: string(dict, kCGWindowOwnerName).unwrap_or_default(),
                bounds,
            });
        }
        CFRelease(list);
    }
    out
}

pub fn foreground_app() -> Option<ForegroundApp> {
    let me = std::process::id() as i32;
    windows()
        .into_iter()
        .find(|w| w.layer == 0 && w.bounds.is_some_and(|b| b.width > 1.0 && b.height > 1.0))
        .map(|w| ForegroundApp { name: w.owner, is_self: w.pid == me, bounds: w.bounds })
}

pub fn running_apps() -> Vec<String> {
    let me = std::process::id() as i32;
    windows()
        .into_iter()
        .filter(|w| w.layer == 0 && w.pid != me && !w.owner.is_empty())
        .map(|w| w.owner)
        .collect()
}
