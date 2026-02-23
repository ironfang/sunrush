use std::ffi::c_char;
use abi::{host_contract::HostApi, plugin_contract::PluginApi};

#[unsafe(no_mangle)]
pub static PLUGIN_API: PluginApi = PluginApi {
    get_name,
    load,
    unload,
};

extern "C" fn get_name() -> *const c_char {
    // static string lives forever → safe
    static NAME: &str = "SunRush Demo Привет";
    NAME.as_ptr() as *const c_char
}

extern "C" fn load(_host: *const HostApi) {
    // TODO: store host pointer and initialise plugin state
}

extern "C" fn unload() {
    // TODO: release plugin resources
}