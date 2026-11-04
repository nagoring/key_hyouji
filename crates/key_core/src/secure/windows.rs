use std::cell::RefCell;
use windows::core::Interface;
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED};
use windows::Win32::UI::Accessibility::{CUIAutomation8, IUIAutomation, IUIAutomation2};

/// 応答しないアプリで待たされないよう UI Automation の待ち時間を短くする（ms）
const UIA_TIMEOUT_MS: u32 = 200;

thread_local! {
    static UIA: RefCell<Option<IUIAutomation>> = const { RefCell::new(None) };
}

fn automation() -> Option<IUIAutomation> {
    UIA.with(|cell| {
        if let Some(a) = cell.borrow().as_ref() {
            return Some(a.clone());
        }
        unsafe {
            // 既に初期化済み（S_FALSE / RPC_E_CHANGED_MODE）でも続行できる
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let a: IUIAutomation = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER).ok()?;
            if let Ok(a2) = a.cast::<IUIAutomation2>() {
                let _ = a2.SetConnectionTimeout(UIA_TIMEOUT_MS);
                let _ = a2.SetTransactionTimeout(UIA_TIMEOUT_MS);
            }
            *cell.borrow_mut() = Some(a.clone());
            Some(a)
        }
    })
}

pub fn password_field_focused() -> bool {
    let Some(a) = automation() else { return false };
    unsafe {
        a.GetFocusedElement()
            .and_then(|el| el.CurrentIsPassword())
            .map(|b| b.as_bool())
            .unwrap_or(false)
    }
}

pub fn secure_input_enabled() -> bool {
    false
}
