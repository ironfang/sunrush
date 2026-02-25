use std::ffi::c_char;
use abi::{host_contract::HostApi, plugin_contract::PluginApi};
use sb::messages::TestPayload;

#[unsafe(no_mangle)]
pub static PLUGIN_API: PluginApi = PluginApi {
    get_name,
    load,
    unload,
};

extern "C" fn get_name() -> *const c_char {
    // c"..." literals are null-terminated; .as_ptr() is valid for 'static.
    c"SunRush Demo Привет".as_ptr()
}

extern "C" fn load(host: *const HostApi) {
    // SAFETY: the host guarantees the pointer is valid and non-null for the
    // lifetime of the `load` call.
    let host = unsafe { &*host };

    let payload = TestPayload::new(1, 3.14, 2.71, "hello from test_plugin");
    host.publish(payload);
}

extern "C" fn unload() {
    // TODO: release plugin resources
}