use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::Input::Ime::ImmGetDefaultIMEWnd;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, SendMessageTimeoutW, SMTO_ABORTIFHUNG, WM_IME_CONTROL,
};

const IMC_GETCONVERSIONMODE: usize = 0x0001;
const IMC_GETOPENSTATUS: usize = 0x0005;
const IME_CMODE_NATIVE: usize = 0x0001;
const TIMEOUT_MS: u32 = 50;

fn ime_control(hwnd: windows::Win32::Foundation::HWND, command: usize) -> Option<usize> {
    let mut result: usize = 0;
    let ok = unsafe {
        SendMessageTimeoutW(
            hwnd,
            WM_IME_CONTROL,
            WPARAM(command),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            TIMEOUT_MS,
            Some(&mut result),
        )
    };
    (ok.0 != 0).then_some(result)
}

pub fn ime_active() -> bool {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() {
            return false;
        }
        let ime = ImmGetDefaultIMEWnd(fg);
        if ime.0.is_null() {
            return false;
        }
        let open = ime_control(ime, IMC_GETOPENSTATUS).unwrap_or(0) != 0;
        if !open {
            return false;
        }
        ime_control(ime, IMC_GETCONVERSIONMODE).is_some_and(|m| m & IME_CMODE_NATIVE != 0)
    }
}
