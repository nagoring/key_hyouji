use super::{ForegroundApp, Rect};
use windows::core::{BOOL, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT};
use windows::Win32::System::Threading::{
    GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetForegroundWindow, GetWindow, GetWindowRect, GetWindowTextLengthW,
    GetWindowThreadProcessId, IsWindowVisible, GW_OWNER,
};

fn process_id(hwnd: HWND) -> u32 {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    pid
}

fn exe_name(pid: u32) -> Option<String> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(handle);
        ok.ok()?;
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        path.rsplit(['\\', '/']).next().map(str::to_owned)
    }
}

pub fn foreground_app() -> Option<ForegroundApp> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let pid = process_id(hwnd);
        let mut r = RECT::default();
        let bounds = GetWindowRect(hwnd, &mut r).ok().map(|_| Rect {
            x: r.left as f64,
            y: r.top as f64,
            width: (r.right - r.left) as f64,
            height: (r.bottom - r.top) as f64,
        });
        Some(ForegroundApp {
            name: exe_name(pid).unwrap_or_default(),
            is_self: pid == GetCurrentProcessId(),
            bounds,
        })
    }
}

unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let pids = &mut *(lparam.0 as *mut Vec<u32>);
    let owner = GetWindow(hwnd, GW_OWNER).unwrap_or_default();
    if IsWindowVisible(hwnd).as_bool() && GetWindowTextLengthW(hwnd) > 0 && owner.0.is_null() {
        let pid = process_id(hwnd);
        if !pids.contains(&pid) {
            pids.push(pid);
        }
    }
    BOOL(1)
}

pub fn running_apps() -> Vec<String> {
    let mut pids: Vec<u32> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(collect), LPARAM(&mut pids as *mut _ as isize));
    }
    let me = unsafe { GetCurrentProcessId() };
    pids.into_iter().filter(|&p| p != me).filter_map(exe_name).collect()
}
