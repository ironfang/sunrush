use std::ffi::{CStr, CString, c_void};
use abi::{
    host_contract::{BusCallback, HostApi},
    plugin_contract::PluginApi,
};
use sb::{Bus, BusMessage};
use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// Host-side callbacks exposed to plugins via HostApi
// ---------------------------------------------------------------------------

extern "C" fn host_print(_ctx: *mut c_void, msg: *const std::ffi::c_char) {
    let s = unsafe { CStr::from_ptr(msg) }.to_string_lossy();
    println!("[plugin] {}", s);
}

/// Publish bytes on a topic.  `bus_ctx` is a `*mut Bus` cast to `*mut c_void`.
extern "C" fn bus_publish(
    bus_ctx: *mut c_void,
    topic: *const std::ffi::c_char,
    data: *const u8,
    len: usize,
) {
    let bus = unsafe { &*(bus_ctx as *const Bus) };
    let topic_str = unsafe { CStr::from_ptr(topic) }.to_str().unwrap_or("");
    let payload = unsafe { std::slice::from_raw_parts(data, len) }.to_vec();
    bus.publisher(topic_str).publish(payload);
}

/// Subscribe to a topic.  The callback is invoked from a dedicated async task.
extern "C" fn bus_subscribe(
    bus_ctx: *mut c_void,
    topic: *const std::ffi::c_char,
    callback: BusCallback,
) {
    let bus = unsafe { &*(bus_ctx as *const Bus) };
    let topic_owned = unsafe { CStr::from_ptr(topic) }
        .to_str()
        .unwrap_or("")
        .to_owned();

    bus.subscribe(topic_owned.clone(), move |msg: BusMessage| {
        let topic_owned = topic_owned.clone();
        async move {
            // Both pointers are valid only inside this async block — safe.
            let topic_cstr = CString::new(topic_owned).unwrap();
            callback(topic_cstr.as_ptr(), msg.data.as_ptr(), msg.data.len());
        }
    });
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    // The Bus is heap-allocated; its raw pointer is passed to plugins as bus_ctx.
    let bus = Box::new(Bus::new());
    let bus_ptr = Box::into_raw(bus);

    let host_api = HostApi {
        host_ctx: std::ptr::null_mut(),
        host_print,
        bus_ctx: bus_ptr as *mut c_void,
        bus_publish,
        bus_subscribe,
    };

    unsafe {
        let lib = Library::new("./target/debug/libtest_plugin.so").unwrap();

        let api: Symbol<*const PluginApi> = lib.get(b"PLUGIN_API").unwrap();
        let api = &**api;

        let name = CStr::from_ptr((api.get_name)()).to_string_lossy();
        println!("Loaded plugin: {}", name);

        (api.load)(&host_api as *const HostApi);

        // ... plugin does its work here ...

        (api.unload)();

        std::mem::forget(lib);

        // Reclaim the Bus after all plugins are unloaded.
        drop(Box::from_raw(bus_ptr));
    }
}
