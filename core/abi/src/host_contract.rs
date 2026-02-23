use std::os::raw::{c_char, c_void};

#[repr(C)]
pub struct HostApi {
    pub host_ctx: *mut c_void,
    pub host_print: extern "C" fn(*mut c_void, *const c_char),
}

