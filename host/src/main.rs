use std::ffi::CStr;
use std::mem::size_of;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use abi::{
    host_contract::HostApi,
    plugin_contract::PluginApi,
};
use libloading::{Library, Symbol};
use sb::messages::TestPayload;
use host_contract_impl::init;

mod host_contract_impl;

static MSG_COUNT:  AtomicU64 = AtomicU64::new(0);
static BYTE_COUNT: AtomicU64 = AtomicU64::new(0);

#[tokio::main]
async fn main() {
    let host_api = init();

    // Count every arriving TestPayload for bandwidth stats.
    host_api.subscribe(|_event: TestPayload| async move {
        MSG_COUNT .fetch_add(1,                             Ordering::Relaxed);
        BYTE_COUNT.fetch_add(size_of::<TestPayload>() as u64, Ordering::Relaxed);
    });

    // Print msg/s and B/s every second.
    tokio::spawn(async {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        interval.tick().await; // first tick fires immediately — skip it
        loop {
            interval.tick().await;
            let msgs  = MSG_COUNT .swap(0, Ordering::Relaxed);
            let bytes = BYTE_COUNT.swap(0, Ordering::Relaxed);
            println!(
                "[bandwidth] {:>10} msg/s  |  {:>12} B/s  ({:.2} MB/s)",
                msgs,
                bytes,
                bytes as f64 / 1_000_000.0,
            );
        }
    });

    println!("Starting host...");

    unsafe {
        let lib = Library::new("./target/debug/libtest_plugin.so").unwrap();

        let api: Symbol<*const PluginApi> = lib.get(b"PLUGIN_API").unwrap();
        let api = &**api;

        let name = CStr::from_ptr((api.get_name)()).to_string_lossy();
        println!("Loaded plugin: {}", name);

        (api.load)(&host_api as *const HostApi);

        // Two yields are needed:
        //   1st — lets the bus subscriber task receive the message and call
        //         dispatch_trampoline (which calls tokio::spawn for the handler).
        //   2nd — lets that newly spawned handler task actually run.
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        println!("Host running. Press Ctrl+C to stop.");
        tokio::signal::ctrl_c().await.expect("failed to listen for Ctrl+C");
        println!("Shutting down...");

        (api.unload)();

        std::mem::forget(lib);
    }
}
