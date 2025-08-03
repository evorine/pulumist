pub mod events;
pub mod outputs;
pub mod proto;
pub mod config;
pub mod engine;
pub mod error;
pub mod stack;
pub mod dynamic;

use std::os::raw::c_char;

// FFI bindings to Go functions
unsafe extern "C" {
    fn PulumiDynamicPreview(request: *const c_char, request_len: i32) -> *mut c_char;
    fn PulumiDynamicDeploy(request: *const c_char, request_len: i32) -> *mut c_char;
    fn PulumiDynamicDestroy(request: *const c_char, request_len: i32) -> *mut c_char;
    fn PulumiDynamicGetOutputs(request: *const c_char, request_len: i32) -> *mut c_char;
    fn PulumiDynamicRefresh(request: *const c_char, request_len: i32) -> *mut c_char;
    fn FreeAllocation(s: *mut c_char);
    fn RegisterEventCallback(callback: Option<unsafe extern "C" fn(*const c_char)>);
    fn UnregisterEventCallback();
}

pub fn nop() {
}