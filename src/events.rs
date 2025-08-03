use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::Mutex;
use serde_json::Value;

lazy_static::lazy_static! {
    static ref EVENT_SENDER: Mutex<Option<Sender<Value>>> = Mutex::new(None);
}

/// FFI callback function that receives events from Go
pub unsafe extern "C" fn event_callback(event_json: *const c_char) {
    if event_json.is_null() {
        return;
    }

    let event_str: &str;
    unsafe {
        event_str = match CStr::from_ptr(event_json).to_str() {
            Ok(s) => s,
            Err(_) => return,
        };
    }

    let event_value: Value = match serde_json::from_str(event_str) {
        Ok(v) => v,
        Err(_) => return,
    };

    // Send event through channel if available
    if let Ok(sender_guard) = EVENT_SENDER.lock() {
        if let Some(sender) = &*sender_guard {
            let _ = sender.send(event_value);
        }
    }
}

/// Creates an event channel and registers the callback
pub fn create_event_channel() -> Receiver<Value> {
    let (sender, receiver) = channel();
    
    // Store the sender
    if let Ok(mut sender_guard) = EVENT_SENDER.lock() {
        *sender_guard = Some(sender);
    }
    
    // Register the callback with Go
    unsafe {
        super::RegisterEventCallback(Some(event_callback));
    }
    
    receiver
}

/// Unregisters the event callback
pub fn cleanup_event_channel() {
    // Clear the sender
    if let Ok(mut sender_guard) = EVENT_SENDER.lock() {
        *sender_guard = None;
    }
    
    // Unregister the callback
    unsafe {
        super::UnregisterEventCallback();
    }
}