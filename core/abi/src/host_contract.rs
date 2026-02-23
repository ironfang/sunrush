use std::os::raw::{c_char, c_void};

pub type BusCallback = extern "C" fn(topic: *const c_char, data: *const u8, len: usize);

#[repr(C)]
pub struct HostApi {
    pub host_ctx: *mut c_void,
    pub host_print: extern "C" fn(ctx: *mut c_void, msg: *const c_char),

    pub bus_ctx: *mut c_void,
    pub bus_publish:
        extern "C" fn(bus_ctx: *mut c_void, topic: *const c_char, data: *const u8, len: usize),
        
    pub bus_subscribe:
        extern "C" fn(bus_ctx: *mut c_void, topic: *const c_char, callback: BusCallback),
}
