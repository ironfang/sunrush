use std::ffi::c_char;

pub struct PluginApi {
    pub get_name: extern "C" fn() -> *const c_char,
}