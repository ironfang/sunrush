use std::ffi::c_char;
use std::sync::atomic::{AtomicBool, Ordering};
use abi::{host_contract::HostApi, plugin_contract::PluginApi};
use sb::messages::TestPayload;

#[unsafe(no_mangle)]
pub static PLUGIN_API: PluginApi = PluginApi {
    get_name,
    load,
    unload,
};

/// Signals the publisher thread to stop.
static STOP: AtomicBool = AtomicBool::new(false);

extern "C" fn get_name() -> *const c_char {
    c"SunRush Demo Привет".as_ptr()
}

extern "C" fn load(host: *const HostApi) {
    STOP.store(false, Ordering::SeqCst);
    // Cast to usize so the closure captures a Send integer rather than a raw
    // pointer (Rust 2024 captures individual fields, so *const T is !Send).
    let host_ptr = host as usize;
    std::thread::spawn(move || {
        let host = unsafe { &*(host_ptr as *const HostApi) };
        let mut seq: u32 = 0;
        while !STOP.load(Ordering::Relaxed) {
            host.publish(TestPayload::new(seq, 0.0, 0.0, "bw"));
            seq = seq.wrapping_add(1);
        }
    });
}

extern "C" fn unload() {
    STOP.store(true, Ordering::SeqCst);
}