use std::ffi::c_char;
use abi::plugin_contract::PluginApi;

#[unsafe(no_mangle)]
pub static PLUGIN_API: PluginApi = PluginApi {
    get_name,
};

extern "C" fn get_name() -> *const c_char {
    // static строка живёт вечно → безопасно
    static NAME: &str = "SunRush Demo Привет";

    NAME.as_ptr() as *const c_char
}