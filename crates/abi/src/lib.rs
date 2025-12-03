use std::os::raw::{c_char, c_void};

/// Current ABI version
pub const ABI_VERSION: u32 = 1;

/// Host callbacks provided to plugins
#[repr(C)]
pub struct HostCallbacks {
    // Logging callbacks
    pub log_trace: extern "C" fn(*const c_char, *const c_char),
    pub log_debug: extern "C" fn(*const c_char, *const c_char),
    pub log_info: extern "C" fn(*const c_char, *const c_char),
    pub log_warn: extern "C" fn(*const c_char, *const c_char),
    pub log_error: extern "C" fn(*const c_char, *const c_char),
    
    // Message bus callbacks
    pub publish: extern "C" fn(u8, *const c_void, usize) -> i32,
    pub subscribe: extern "C" fn(u8) -> *mut c_void,
    pub recv: extern "C" fn(*mut c_void, *mut c_void, *mut usize, u64) -> i32,
    pub unsubscribe: extern "C" fn(*mut c_void),
    
    // Metrics callbacks
    pub counter_inc: extern "C" fn(*const c_char),
    pub counter_add: extern "C" fn(*const c_char, u64),
    pub gauge_set: extern "C" fn(*const c_char, f64),
    pub histogram_observe: extern "C" fn(*const c_char, f64),
    
    // Configuration callback
    pub get_config: extern "C" fn(*const c_char) -> *const c_char,
    
    // Utility callbacks
    pub now_nanos: extern "C" fn() -> u64,
    pub sleep_ms: extern "C" fn(u64),
    
    // Reserved for future use
    pub _reserved: [*mut c_void; 8],
}

/// Plugin exports (function signatures)
pub type InitFn = unsafe extern "C" fn(*const HostCallbacks, *const c_char) -> *mut c_void;
pub type StartFn = unsafe extern "C" fn(*mut c_void) -> i32;
pub type StopFn = unsafe extern "C" fn(*mut c_void) -> i32;
pub type CleanupFn = unsafe extern "C" fn(*mut c_void);
pub type InfoFn = unsafe extern "C" fn() -> *const c_char;
pub type AbiVersionFn = unsafe extern "C" fn() -> u32;

/// Plugin export symbol names
pub const INIT_SYMBOL: &[u8] = b"mef_component_init\0";
pub const START_SYMBOL: &[u8] = b"mef_component_start\0";
pub const STOP_SYMBOL: &[u8] = b"mef_component_stop\0";
pub const CLEANUP_SYMBOL: &[u8] = b"mef_component_cleanup\0";
pub const INFO_SYMBOL: &[u8] = b"mef_component_info\0";
pub const ABI_VERSION_SYMBOL: &[u8] = b"mef_component_abi_version\0";
