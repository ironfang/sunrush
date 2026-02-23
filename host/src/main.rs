use std::ffi::CStr;
use abi::plugin_contract::PluginApi;
use libloading::{Library, Symbol};

fn main() {
    unsafe {
        let lib = Library::new("./target/debug/libtest_plugin.so").unwrap();

        let api: Symbol<*const PluginApi> =
            lib.get(b"PLUGIN_API").unwrap();

        let api = &**api;

        let name_ptr = (api.get_name)();

        let name = CStr::from_ptr(name_ptr)
            .to_string_lossy();

        println!("Loaded plugin: {}", name);

        std::mem::forget(lib);
    }
}
