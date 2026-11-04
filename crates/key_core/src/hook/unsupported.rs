use super::HookError;
use crate::event::KeyEvent;
use std::sync::mpsc::Sender;

pub fn start(_sink: Sender<KeyEvent>) -> Result<(), HookError> {
    Err(HookError::Failed("this platform is not supported".into()))
}

pub fn has_permission() -> bool {
    true
}

pub fn request_permission() {}
