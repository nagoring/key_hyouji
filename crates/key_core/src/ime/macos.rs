use std::ffi::{c_char, c_void, CStr};

type CFTypeRef = *const c_void;

#[link(name = "Carbon", kind = "framework")]
extern "C" {
    static kTISPropertyInputSourceID: CFTypeRef;
    static kTISPropertyInputSourceType: CFTypeRef;
    static kTISTypeKeyboardInputMode: CFTypeRef;
    fn TISCopyCurrentKeyboardInputSource() -> CFTypeRef;
    fn TISGetInputSourceProperty(source: CFTypeRef, key: CFTypeRef) -> CFTypeRef;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFEqual(a: CFTypeRef, b: CFTypeRef) -> u8;
    fn CFStringGetCString(s: CFTypeRef, buf: *mut c_char, size: isize, encoding: u32) -> bool;
    fn CFRelease(cf: CFTypeRef);
}

const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;

pub fn ime_active() -> bool {
    unsafe {
        let source = TISCopyCurrentKeyboardInputSource();
        if source.is_null() {
            return false;
        }
        let ty = TISGetInputSourceProperty(source, kTISPropertyInputSourceType);
        let is_mode = !ty.is_null() && CFEqual(ty, kTISTypeKeyboardInputMode) != 0;
        let id_ref = TISGetInputSourceProperty(source, kTISPropertyInputSourceID);
        let mut buf = [0 as c_char; 256];
        let id = if !id_ref.is_null()
            && CFStringGetCString(id_ref, buf.as_mut_ptr(), buf.len() as isize, K_CF_STRING_ENCODING_UTF8)
        {
            CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned()
        } else {
            String::new()
        };
        CFRelease(source);
        super::is_ime_source(&id, is_mode)
    }
}
