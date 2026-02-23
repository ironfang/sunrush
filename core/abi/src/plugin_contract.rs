use std::ffi::c_char;
use crate::host_contract::HostApi;

pub struct PluginApi {
    pub get_name: extern "C" fn() -> *const c_char,
    pub load: extern "C" fn(host: *const HostApi),
    pub unload: extern "C" fn(),
}