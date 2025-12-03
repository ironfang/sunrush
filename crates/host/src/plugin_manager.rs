use libloading::{Library, Symbol};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::ffi::{CString, CStr};
use std::os::raw::{c_char, c_void};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use sunrush_abi::*;
use sunrush_bus::MessageBus;
use crate::telemetry::TelemetryServer;
use tracing::{info, error, warn};

pub struct PluginManager {
    plugins: RwLock<HashMap<String, Plugin>>,
    callbacks: Arc<HostCallbacks>,
    bus: Arc<MessageBus>,
    telemetry: Arc<TelemetryServer>,
    plugin_dir: PathBuf,
}

struct Plugin {
    name: String,
    path: PathBuf,
    library: Library,
    handle: *mut c_void,
    start_fn: StartFn,
    stop_fn: StopFn,
    cleanup_fn: CleanupFn,
    running: bool,
}

unsafe impl Send for Plugin {}
unsafe impl Sync for Plugin {}

impl PluginManager {
    pub fn new(
        bus: Arc<MessageBus>,
        telemetry: Arc<TelemetryServer>,
        plugin_dir: PathBuf,
        config: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    ) -> Arc<Self> {
        let callbacks = Arc::new(Self::create_callbacks(
            Arc::clone(&bus),
            Arc::clone(&telemetry),
            Arc::clone(&config),
        ));

        Arc::new(Self {
            plugins: RwLock::new(HashMap::new()),
            callbacks,
            bus,
            telemetry,
            plugin_dir,
        })
    }

    fn create_callbacks(
        _bus: Arc<MessageBus>,
        _telemetry: Arc<TelemetryServer>,
        _config: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    ) -> HostCallbacks {
        HostCallbacks {
            log_trace: host_log_trace,
            log_debug: host_log_debug,
            log_info: host_log_info,
            log_warn: host_log_warn,
            log_error: host_log_error,
            publish: host_publish,
            subscribe: host_subscribe,
            recv: host_recv,
            unsubscribe: host_unsubscribe,
            counter_inc: host_counter_inc,
            counter_add: host_counter_add,
            gauge_set: host_gauge_set,
            histogram_observe: host_histogram_observe,
            get_config: host_get_config,
            now_nanos: host_now_nanos,
            sleep_ms: host_sleep_ms,
            _reserved: [std::ptr::null_mut(); 8],
        }
    }

    pub fn load_plugin(&self, path: &Path) -> anyhow::Result<()> {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid plugin filename"))?
            .to_string();

        info!("Loading plugin: {} from {:?}", name, path);

        // Load library
        let library = unsafe { Library::new(path)? };

        // Get ABI version
        let abi_version_fn: Symbol<AbiVersionFn> =
            unsafe { library.get(ABI_VERSION_SYMBOL)? };
        let abi_version = unsafe { abi_version_fn() };

        if abi_version != ABI_VERSION {
            anyhow::bail!(
                "ABI version mismatch: plugin={}, host={}",
                abi_version,
                ABI_VERSION
            );
        }

        // Get required symbols and convert to function pointers immediately
        let init_fn: Symbol<InitFn> = unsafe { library.get(INIT_SYMBOL)? };
        let init_fn_ptr: InitFn = *init_fn;
        let start_fn: Symbol<StartFn> = unsafe { library.get(START_SYMBOL)? };
        let start_fn_ptr: StartFn = *start_fn;
        let stop_fn: Symbol<StopFn> = unsafe { library.get(STOP_SYMBOL)? };
        let stop_fn_ptr: StopFn = *stop_fn;
        let cleanup_fn: Symbol<CleanupFn> = unsafe { library.get(CLEANUP_SYMBOL)? };
        let cleanup_fn_ptr: CleanupFn = *cleanup_fn;

        // Initialize plugin
        let config_json = CString::new("{}").unwrap();
        let handle = unsafe {
            init_fn_ptr(
                &*self.callbacks as *const HostCallbacks,
                config_json.as_ptr(),
            )
        };

        if handle.is_null() {
            anyhow::bail!("Plugin initialization failed");
        }

        let plugin = Plugin {
            name: name.clone(),
            path: path.to_path_buf(),
            library,
            handle,
            start_fn: start_fn_ptr,
            stop_fn: stop_fn_ptr,
            cleanup_fn: cleanup_fn_ptr,
            running: false,
        };

        self.plugins.write().insert(name.clone(), plugin);
        info!("Plugin {} loaded successfully", name);

        Ok(())
    }

    pub fn start_plugin(&self, name: &str) -> anyhow::Result<()> {
        let mut plugins = self.plugins.write();
        let plugin = plugins
            .get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", name))?;

        if plugin.running {
            return Ok(());
        }

        info!("Starting plugin: {}", name);
        let result = unsafe { (plugin.start_fn)(plugin.handle) };

        if result != 0 {
            anyhow::bail!("Plugin start failed with code: {}", result);
        }

        plugin.running = true;
        info!("Plugin {} started successfully", name);

        Ok(())
    }

    pub fn stop_plugin(&self, name: &str) -> anyhow::Result<()> {
        let mut plugins = self.plugins.write();
        let plugin = plugins
            .get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", name))?;

        if !plugin.running {
            return Ok(());
        }

        info!("Stopping plugin: {}", name);
        let result = unsafe { (plugin.stop_fn)(plugin.handle) };

        if result != 0 {
            warn!("Plugin stop returned non-zero code: {}", result);
        }

        plugin.running = false;
        info!("Plugin {} stopped", name);

        Ok(())
    }

    pub fn load_all_plugins(&self) -> anyhow::Result<()> {
        let entries = std::fs::read_dir(&self.plugin_dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("so") {
                if let Err(e) = self.load_plugin(&path) {
                    error!("Failed to load plugin {:?}: {}", path, e);
                }
            }
        }

        Ok(())
    }

    pub fn start_all_plugins(&self) -> anyhow::Result<()> {
        let names: Vec<String> = self.plugins.read().keys().cloned().collect();

        for name in names {
            if let Err(e) = self.start_plugin(&name) {
                error!("Failed to start plugin {}: {}", name, e);
            }
        }

        Ok(())
    }

    pub fn stop_all_plugins(&self) -> anyhow::Result<()> {
        let names: Vec<String> = self.plugins.read().keys().cloned().collect();

        for name in names {
            if let Err(e) = self.stop_plugin(&name) {
                error!("Failed to stop plugin {}: {}", name, e);
            }
        }

        Ok(())
    }
}

// Host callback implementations
extern "C" fn host_log_trace(component: *const c_char, msg: *const c_char) {
    let comp = unsafe { CStr::from_ptr(component).to_string_lossy() };
    let message = unsafe { CStr::from_ptr(msg).to_string_lossy() };
    tracing::trace!(component = %comp, "{}", message);
}

extern "C" fn host_log_debug(component: *const c_char, msg: *const c_char) {
    let comp = unsafe { CStr::from_ptr(component).to_string_lossy() };
    let message = unsafe { CStr::from_ptr(msg).to_string_lossy() };
    tracing::debug!(component = %comp, "{}", message);
}

extern "C" fn host_log_info(component: *const c_char, msg: *const c_char) {
    let comp = unsafe { CStr::from_ptr(component).to_string_lossy() };
    let message = unsafe { CStr::from_ptr(msg).to_string_lossy() };
    tracing::info!(component = %comp, "{}", message);
}

extern "C" fn host_log_warn(component: *const c_char, msg: *const c_char) {
    let comp = unsafe { CStr::from_ptr(component).to_string_lossy() };
    let message = unsafe { CStr::from_ptr(msg).to_string_lossy() };
    tracing::warn!(component = %comp, "{}", message);
}

extern "C" fn host_log_error(component: *const c_char, msg: *const c_char) {
    let comp = unsafe { CStr::from_ptr(component).to_string_lossy() };
    let message = unsafe { CStr::from_ptr(msg).to_string_lossy() };
    tracing::error!(component = %comp, "{}", message);
}

extern "C" fn host_publish(_msg_type: u8, _data: *const c_void, _len: usize) -> i32 {
    // TODO: Implement actual publish logic
    0
}

extern "C" fn host_subscribe(_msg_type: u8) -> *mut c_void {
    // TODO: Implement actual subscribe logic
    std::ptr::null_mut()
}

extern "C" fn host_recv(
    _subscription: *mut c_void,
    _buffer: *mut c_void,
    _len: *mut usize,
    _timeout_ms: u64,
) -> i32 {
    // TODO: Implement actual recv logic
    -1
}

extern "C" fn host_unsubscribe(_subscription: *mut c_void) {
    // TODO: Implement actual unsubscribe logic
}

extern "C" fn host_counter_inc(name: *const c_char) {
    // TODO: Implement with actual telemetry
    let _ = unsafe { CStr::from_ptr(name).to_string_lossy() };
}

extern "C" fn host_counter_add(name: *const c_char, _value: u64) {
    let _ = unsafe { CStr::from_ptr(name).to_string_lossy() };
}

extern "C" fn host_gauge_set(name: *const c_char, _value: f64) {
    let _ = unsafe { CStr::from_ptr(name).to_string_lossy() };
}

extern "C" fn host_histogram_observe(name: *const c_char, _value: f64) {
    let _ = unsafe { CStr::from_ptr(name).to_string_lossy() };
}

extern "C" fn host_get_config(_key: *const c_char) -> *const c_char {
    // TODO: Implement config lookup
    std::ptr::null()
}

extern "C" fn host_now_nanos() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

extern "C" fn host_sleep_ms(ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(ms));
}
