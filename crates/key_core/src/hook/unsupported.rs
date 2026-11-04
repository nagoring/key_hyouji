use super::HookError;
use crate::event::InputEvent;
use std::sync::mpsc::Sender;

pub fn start(_sink: Sender<InputEvent>) -> Result<(), HookError> {
    Err(HookError::Failed("this platform is not supported".into()))
}

pub fn has_permission() -> bool {
    true
}

pub fn request_permission() {}

pub fn set_mouse_enabled(_enabled: bool) {}
