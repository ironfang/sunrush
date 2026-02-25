use std::ffi::CStr;
use abi::{
    host_contract::HostApi,
    plugin_contract::PluginApi,
};
use libloading::{Library, Symbol};
use sb::messages::TestPayload;
use host_contract_impl::init;

mod host_contract_impl;

#[tokio::main]
async fn main() {
    let host_api = init();

    // Subscribe before any plugin is loaded so no events are missed.
    host_api.subscribe(|event: TestPayload| async move {
        println!(
            "[host] received TestPayload: id={} x={:.2} y={:.2} name=\"{}\"",
            event.id, event.x, event.y, event.name()
        );
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
