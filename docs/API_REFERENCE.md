# SunRush ABI Reference

**Version:** 1.0  
**ABI Version:** 1  
**Last Updated:** December 3, 2025

## Table of Contents

1. [Overview](#overview)
2. [ABI Stability](#abi-stability)
3. [Plugin Exports](#plugin-exports)
4. [Host Callbacks](#host-callbacks)
5. [Data Types](#data-types)
6. [Message Protocol](#message-protocol)
7. [Error Codes](#error-codes)
8. [Example Implementation](#example-implementation)

---

## Overview

The SunRush ABI (Application Binary Interface) defines the contract between the host process and dynamically-loaded plugins. It ensures binary compatibility across different plugin versions and compilation units.

### ABI Version

```c
#define SUNRUSH_ABI_VERSION 1
```

The host verifies this version when loading plugins. Mismatches result in load failure.

### Language Support

While SunRush is written in Rust, the ABI is C-compatible and can be used from:
- Rust (primary)
- C/C++
- Zig
- Any language with C FFI support

---

## ABI Stability

### Stability Guarantees

The SunRush ABI provides the following guarantees:

1. **Binary Compatibility**: Plugins compiled against ABI v1 will work with any host implementing ABI v1
2. **Forward Compatibility**: New callbacks may be added in future versions
3. **Deprecation Policy**: Callbacks are never removed, only deprecated
4. **Layout Stability**: Struct layouts are fixed and padding is explicit

### Breaking Changes

Breaking changes require an ABI version bump:
- Changing callback signatures
- Reordering struct fields
- Changing data type sizes
- Removing callbacks

---

## Plugin Exports

Every plugin must export the following C functions:

### mef_component_init

Initialize the plugin.

```c
void* mef_component_init(
    const HostCallbacks* callbacks,
    const char* config_json
);
```

**Parameters:**
- `callbacks`: Pointer to host callback structure (valid for plugin lifetime)
- `config_json`: Null-terminated JSON configuration string

**Returns:**
- Opaque plugin handle (passed to other functions)
- `NULL` on initialization failure

**Notes:**
- Called exactly once after plugin load
- Must not block
- Store the `callbacks` pointer for later use

**Example:**
```rust
#[no_mangle]
pub extern "C" fn mef_component_init(
    callbacks: *const HostCallbacks,
    config: *const c_char,
) -> *mut c_void {
    let host = unsafe { HostHandle::from_raw(callbacks) };
    let config_str = unsafe { CStr::from_ptr(config).to_str().unwrap() };
    
    let plugin = Box::new(MyPlugin::new(host, config_str));
    Box::into_raw(plugin) as *mut c_void
}
```

---

### mef_component_start

Start the plugin's main processing.

```c
int mef_component_start(void* handle);
```

**Parameters:**
- `handle`: Plugin handle returned from `mef_component_init`

**Returns:**
- `0`: Success (plugin running)
- Non-zero: Error code (see [Error Codes](#error-codes))

**Notes:**
- Called after initialization
- Should spawn background tasks and return quickly
- Blocking is acceptable for up to 5 seconds

**Example:**
```rust
#[no_mangle]
pub extern "C" fn mef_component_start(handle: *mut c_void) -> i32 {
    let plugin = unsafe { &mut *(handle as *mut MyPlugin) };
    
    match plugin.start() {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("Start failed: {}", e);
            -1
        }
    }
}
```

---

### mef_component_stop

Stop the plugin gracefully.

```c
int mef_component_stop(void* handle);
```

**Parameters:**
- `handle`: Plugin handle

**Returns:**
- `0`: Success
- Non-zero: Error code

**Notes:**
- Must complete within 10 seconds
- Should cancel background tasks
- Must flush any pending data
- Called before hot reload or shutdown

**Example:**
```rust
#[no_mangle]
pub extern "C" fn mef_component_stop(handle: *mut c_void) -> i32 {
    let plugin = unsafe { &mut *(handle as *mut MyPlugin) };
    plugin.stop();
    0
}
```

---

### mef_component_cleanup

Free plugin resources.

```c
void mef_component_cleanup(void* handle);
```

**Parameters:**
- `handle`: Plugin handle

**Returns:**
- None

**Notes:**
- Called after `mef_component_stop`
- Must free all allocated memory
- Must not use host callbacks after this point

**Example:**
```rust
#[no_mangle]
pub extern "C" fn mef_component_cleanup(handle: *mut c_void) {
    unsafe {
        let _ = Box::from_raw(handle as *mut MyPlugin);
    }
}
```

---

### mef_component_info

Get plugin metadata.

```c
const char* mef_component_info();
```

**Returns:**
- Pointer to null-terminated JSON string
- String must be static (valid forever)

**Format:**
```json
{
  "name": "plugin-name",
  "version": "1.0.0",
  "description": "Plugin description",
  "author": "Author Name",
  "abi_version": 1
}
```

**Example:**
```rust
#[no_mangle]
pub extern "C" fn mef_component_info() -> *const c_char {
    static INFO: &str = r#"{
        "name": "my-plugin",
        "version": "1.0.0",
        "description": "Example plugin",
        "author": "SunRush Team",
        "abi_version": 1
    }"#;
    INFO.as_ptr() as *const c_char
}
```

---

### mef_component_abi_version

Get ABI version.

```c
uint32_t mef_component_abi_version();
```

**Returns:**
- ABI version number (currently `1`)

**Example:**
```rust
#[no_mangle]
pub extern "C" fn mef_component_abi_version() -> u32 {
    1
}
```

---

## Host Callbacks

The host provides these callbacks to plugins via the `HostCallbacks` structure.

### Structure Definition

```c
typedef struct {
    // Logging callbacks
    void (*log_trace)(const char* component, const char* msg);
    void (*log_debug)(const char* component, const char* msg);
    void (*log_info)(const char* component, const char* msg);
    void (*log_warn)(const char* component, const char* msg);
    void (*log_error)(const char* component, const char* msg);
    
    // Message bus callbacks
    int (*publish)(uint8_t msg_type, const void* data, size_t len);
    void* (*subscribe)(uint8_t msg_type);
    int (*recv)(void* subscription, void* buffer, size_t* len, uint64_t timeout_ms);
    void (*unsubscribe)(void* subscription);
    
    // Metrics callbacks
    void (*counter_inc)(const char* name);
    void (*counter_add)(const char* name, uint64_t value);
    void (*gauge_set)(const char* name, double value);
    void (*histogram_observe)(const char* name, double value);
    
    // Configuration callbacks
    const char* (*get_config)(const char* key);
    
    // Utility callbacks
    uint64_t (*now_nanos)();
    void (*sleep_ms)(uint64_t ms);
    
    // Reserved for future use
    void* _reserved[8];
} HostCallbacks;
```

---

### Logging Callbacks

#### log_trace / log_debug / log_info / log_warn / log_error

```c
void log_info(const char* component, const char* msg);
```

**Parameters:**
- `component`: Component name (null-terminated)
- `msg`: Log message (null-terminated)

**Thread Safety:** Safe

**Example:**
```rust
host.log_info(
    c"my-plugin".as_ptr(),
    c"Processing started".as_ptr()
);
```

---

### Message Bus Callbacks

#### publish

Publish a message to the bus.

```c
int publish(uint8_t msg_type, const void* data, size_t len);
```

**Parameters:**
- `msg_type`: Message type (see [Message Types](#message-types))
- `data`: Pointer to message data
- `len`: Length of message data (bytes)

**Returns:**
- `0`: Success
- `-1`: No receivers
- `-2`: Bus full (backpressure)
- `-3`: Invalid message

**Thread Safety:** Safe

**Example:**
```rust
let tx = MefTransaction { /* ... */ };
let encoded = bincode::serialize(&tx)?;
host.publish(
    MessageType::Transaction as u8,
    encoded.as_ptr() as *const c_void,
    encoded.len()
)?;
```

---

#### subscribe

Subscribe to messages of a specific type.

```c
void* subscribe(uint8_t msg_type);
```

**Parameters:**
- `msg_type`: Message type to subscribe to

**Returns:**
- Subscription handle
- `NULL` on error

**Thread Safety:** Safe

**Notes:**
- Each subscription has an independent queue
- Call `unsubscribe` when done

**Example:**
```rust
let sub = host.subscribe(MessageType::Transaction as u8);
if sub.is_null() {
    return Err("Subscribe failed");
}
```

---

#### recv

Receive a message from subscription.

```c
int recv(
    void* subscription,
    void* buffer,
    size_t* len,
    uint64_t timeout_ms
);
```

**Parameters:**
- `subscription`: Subscription handle
- `buffer`: Buffer to write message data
- `len`: In: buffer size, Out: message size
- `timeout_ms`: Timeout in milliseconds (`0` = non-blocking, `UINT64_MAX` = infinite)

**Returns:**
- `0`: Message received
- `-1`: Timeout
- `-2`: Subscription closed
- `-3`: Buffer too small (check `*len` for required size)

**Thread Safety:** Safe (per subscription)

**Example:**
```rust
let mut buffer = vec![0u8; 4096];
let mut len = buffer.len();

let result = host.recv(
    sub,
    buffer.as_mut_ptr() as *mut c_void,
    &mut len,
    1000  // 1 second timeout
);

if result == 0 {
    let tx: MefTransaction = bincode::deserialize(&buffer[..len])?;
    // Process transaction
}
```

---

#### unsubscribe

Unsubscribe and free resources.

```c
void unsubscribe(void* subscription);
```

**Parameters:**
- `subscription`: Subscription handle

**Thread Safety:** Safe

**Notes:**
- Closes the subscription
- Pending `recv` calls will return `-2`
- Must not use subscription after this call

---

### Metrics Callbacks

#### counter_inc

Increment a counter by 1.

```c
void counter_inc(const char* name);
```

**Parameters:**
- `name`: Metric name (null-terminated)

**Thread Safety:** Safe

**Example:**
```rust
host.counter_inc(c"events_processed_total".as_ptr());
```

---

#### counter_add

Add a value to a counter.

```c
void counter_add(const char* name, uint64_t value);
```

**Parameters:**
- `name`: Metric name
- `value`: Value to add

**Thread Safety:** Safe

---

#### gauge_set

Set a gauge value.

```c
void gauge_set(const char* name, double value);
```

**Parameters:**
- `name`: Metric name
- `value`: Gauge value

**Thread Safety:** Safe

**Example:**
```rust
host.gauge_set(c"queue_size".as_ptr(), 42.0);
```

---

#### histogram_observe

Record a histogram observation.

```c
void histogram_observe(const char* name, double value);
```

**Parameters:**
- `name`: Metric name
- `value`: Observed value

**Thread Safety:** Safe

**Example:**
```rust
let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
host.histogram_observe(c"processing_latency_ms".as_ptr(), latency_ms);
```

---

### Configuration Callbacks

#### get_config

Get configuration value.

```c
const char* get_config(const char* key);
```

**Parameters:**
- `key`: Configuration key (null-terminated)

**Returns:**
- Pointer to value string (null-terminated)
- `NULL` if key not found

**Thread Safety:** Safe

**Notes:**
- Returned pointer is valid until next call
- Do not free the returned pointer

**Example:**
```rust
let endpoint = host.get_config(c"endpoint".as_ptr());
if !endpoint.is_null() {
    let endpoint_str = unsafe { CStr::from_ptr(endpoint).to_str()? };
    // Use endpoint_str
}
```

---

### Utility Callbacks

#### now_nanos

Get current time in nanoseconds.

```c
uint64_t now_nanos();
```

**Returns:**
- Nanoseconds since UNIX epoch

**Thread Safety:** Safe

---

#### sleep_ms

Sleep for specified milliseconds.

```c
void sleep_ms(uint64_t ms);
```

**Parameters:**
- `ms`: Milliseconds to sleep

**Thread Safety:** Safe

---

## Data Types

### Message Types

```c
typedef enum {
    MESSAGE_TYPE_SHRED = 0,
    MESSAGE_TYPE_SLOT_ENTRY = 1,
    MESSAGE_TYPE_BLOCK = 2,
    MESSAGE_TYPE_TRANSACTION = 3,
    MESSAGE_TYPE_CUSTOM_BASE = 100  // Custom types start here
} MessageType;
```

---

### Shred

```rust
#[repr(C)]
pub struct Shred {
    pub slot: u64,
    pub index: u32,
    pub _padding: u32,
    pub data_len: u64,
    pub data_ptr: *const u8,
    pub receive_time_ns: u64,
}
```

**Layout:**
- Total size: 40 bytes
- Alignment: 8 bytes

**Notes:**
- `data_ptr` points to shred data (owned by host)
- Data is valid until next message
- Copy data if needed beyond message lifetime

---

### SlotEntry

```rust
#[repr(C)]
pub struct SlotEntry {
    pub slot: u64,
    pub entry_index: u32,
    pub _padding: u32,
    pub data_len: u64,
    pub data_ptr: *const u8,
}
```

**Layout:**
- Total size: 32 bytes
- Alignment: 8 bytes

---

### Block

```rust
#[repr(C)]
pub struct Block {
    pub slot: u64,
    pub parent_slot: u64,
    pub data_len: u64,
    pub data_ptr: *const u8,
    pub blockhash: [u8; 32],
}
```

**Layout:**
- Total size: 64 bytes
- Alignment: 8 bytes

---

### MefTransaction

```rust
#[repr(C)]
pub struct MefTransaction {
    pub slot: u64,
    pub signature: [u8; 64],
    pub accounts_len: u32,
    pub instructions_len: u32,
    pub accounts_ptr: *const Pubkey,
    pub instructions_ptr: *const Instruction,
    pub raw_message_len: u64,
    pub raw_message_ptr: *const u8,
    pub timestamp_ns: u64,
}
```

**Layout:**
- Total size: 120 bytes
- Alignment: 8 bytes

---

### Pubkey

```rust
#[repr(C)]
pub struct Pubkey {
    pub bytes: [u8; 32],
}
```

---

### Instruction

```rust
#[repr(C)]
pub struct Instruction {
    pub program_id_index: u8,
    pub accounts_len: u8,
    pub data_len: u16,
    pub accounts_ptr: *const u8,
    pub data_ptr: *const u8,
}
```

---

## Message Protocol

### Serialization Format

Messages are serialized using [bincode](https://github.com/bincode-org/bincode) with the following configuration:

```rust
bincode::DefaultOptions::new()
    .with_fixint_encoding()
    .with_little_endian()
```

### Wire Format

```
┌─────────────────────────────────────┐
│ Message Type (1 byte)               │
├─────────────────────────────────────┤
│ Length (4 bytes, little-endian)     │
├─────────────────────────────────────┤
│ Payload (Length bytes)              │
│ (bincode-encoded data)              │
└─────────────────────────────────────┘
```

### Example: Publishing a Transaction

```rust
// 1. Create transaction
let tx = MefTransaction {
    slot: 245123456,
    signature: [0u8; 64],
    // ... other fields
};

// 2. Serialize
let encoded = bincode::serialize(&tx)?;

// 3. Publish
host.publish(
    MessageType::Transaction as u8,
    encoded.as_ptr() as *const c_void,
    encoded.len()
)?;
```

### Example: Receiving a Transaction

```rust
// 1. Subscribe
let sub = host.subscribe(MessageType::Transaction as u8);

// 2. Receive loop
loop {
    let mut buffer = vec![0u8; 8192];
    let mut len = buffer.len();
    
    let result = host.recv(sub, buffer.as_mut_ptr() as *mut c_void, &mut len, 1000);
    
    if result == 0 {
        // 3. Deserialize
        let tx: MefTransaction = bincode::deserialize(&buffer[..len])?;
        
        // 4. Process
        process_transaction(tx);
    }
}
```

---

## Error Codes

### Standard Error Codes

| Code | Name | Description |
|------|------|-------------|
| 0 | `SUCCESS` | Operation succeeded |
| -1 | `ERROR_GENERIC` | Generic error |
| -2 | `ERROR_INVALID_PARAM` | Invalid parameter |
| -3 | `ERROR_NOT_FOUND` | Resource not found |
| -4 | `ERROR_TIMEOUT` | Operation timed out |
| -5 | `ERROR_NO_MEMORY` | Out of memory |
| -6 | `ERROR_IO` | I/O error |
| -7 | `ERROR_BUSY` | Resource busy |
| -8 | `ERROR_WOULD_BLOCK` | Operation would block |

### Plugin-Specific Error Codes

Plugins can define custom error codes starting from -1000:

```c
#define MY_PLUGIN_ERROR_BASE (-1000)
#define ERROR_INVALID_CONFIG (MY_PLUGIN_ERROR_BASE - 1)
#define ERROR_CONNECTION_FAILED (MY_PLUGIN_ERROR_BASE - 2)
```

---

## Example Implementation

### Complete Plugin in Rust

```rust
use std::ffi::{CStr, CString, c_char, c_void};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

// ============================================
// ABI Types
// ============================================

#[repr(C)]
pub struct HostCallbacks {
    pub log_info: extern "C" fn(*const c_char, *const c_char),
    pub publish: extern "C" fn(u8, *const c_void, usize) -> i32,
    pub subscribe: extern "C" fn(u8) -> *mut c_void,
    pub recv: extern "C" fn(*mut c_void, *mut c_void, *mut usize, u64) -> i32,
    pub counter_inc: extern "C" fn(*const c_char),
    // ... other callbacks
}

pub const ABI_VERSION: u32 = 1;

// ============================================
// Plugin Implementation
// ============================================

pub struct MyPlugin {
    callbacks: Arc<HostCallbacks>,
    config: PluginConfig,
}

#[derive(Deserialize)]
struct PluginConfig {
    enabled: bool,
    threshold: u64,
}

impl MyPlugin {
    fn new(callbacks: Arc<HostCallbacks>, config_str: &str) -> Self {
        let config: PluginConfig = serde_json::from_str(config_str)
            .unwrap_or(PluginConfig {
                enabled: true,
                threshold: 100,
            });
        
        Self { callbacks, config }
    }
    
    fn start(&mut self) -> Result<(), String> {
        self.log_info("Plugin starting");
        
        // Subscribe to transactions
        let sub = unsafe {
            (self.callbacks.subscribe)(3)  // MESSAGE_TYPE_TRANSACTION
        };
        
        if sub.is_null() {
            return Err("Subscribe failed".into());
        }
        
        // Spawn processing task
        let callbacks = Arc::clone(&self.callbacks);
        std::thread::spawn(move || {
            Self::process_loop(callbacks, sub);
        });
        
        Ok(())
    }
    
    fn process_loop(callbacks: Arc<HostCallbacks>, sub: *mut c_void) {
        let mut buffer = vec![0u8; 8192];
        
        loop {
            let mut len = buffer.len();
            
            let result = unsafe {
                (callbacks.recv)(
                    sub,
                    buffer.as_mut_ptr() as *mut c_void,
                    &mut len,
                    1000  // 1 second timeout
                )
            };
            
            if result == 0 {
                // Process message
                Self::log_info_static(&callbacks, "Received transaction");
                Self::counter_inc_static(&callbacks, "transactions_processed");
            }
        }
    }
    
    fn log_info(&self, msg: &str) {
        Self::log_info_static(&self.callbacks, msg);
    }
    
    fn log_info_static(callbacks: &HostCallbacks, msg: &str) {
        let component = CString::new("my-plugin").unwrap();
        let message = CString::new(msg).unwrap();
        unsafe {
            (callbacks.log_info)(component.as_ptr(), message.as_ptr());
        }
    }
    
    fn counter_inc_static(callbacks: &HostCallbacks, name: &str) {
        let metric_name = CString::new(name).unwrap();
        unsafe {
            (callbacks.counter_inc)(metric_name.as_ptr());
        }
    }
}

// ============================================
// Plugin Exports
// ============================================

static mut PLUGIN: Option<Box<MyPlugin>> = None;

#[no_mangle]
pub extern "C" fn mef_component_init(
    callbacks: *const HostCallbacks,
    config: *const c_char,
) -> *mut c_void {
    let callbacks = Arc::new(unsafe { *callbacks });
    
    let config_str = unsafe {
        CStr::from_ptr(config).to_str().unwrap_or("{}")
    };
    
    let plugin = Box::new(MyPlugin::new(callbacks, config_str));
    
    unsafe {
        PLUGIN = Some(plugin);
        PLUGIN.as_mut().unwrap().as_mut() as *mut MyPlugin as *mut c_void
    }
}

#[no_mangle]
pub extern "C" fn mef_component_start(_handle: *mut c_void) -> i32 {
    unsafe {
        if let Some(plugin) = &mut PLUGIN {
            match plugin.start() {
                Ok(_) => 0,
                Err(e) => {
                    eprintln!("Start failed: {}", e);
                    -1
                }
            }
        } else {
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn mef_component_stop(_handle: *mut c_void) -> i32 {
    // Cleanup logic here
    0
}

#[no_mangle]
pub extern "C" fn mef_component_cleanup(_handle: *mut c_void) {
    unsafe {
        PLUGIN = None;
    }
}

#[no_mangle]
pub extern "C" fn mef_component_info() -> *const c_char {
    static INFO: &str = r#"{
        "name": "my-plugin",
        "version": "1.0.0",
        "abi_version": 1
    }"#;
    INFO.as_ptr() as *const c_char
}

#[no_mangle]
pub extern "C" fn mef_component_abi_version() -> u32 {
    ABI_VERSION
}
```

---

## Versioning

### ABI Version History

| Version | Release Date | Changes |
|---------|--------------|---------|
| 1 | 2025-12-03 | Initial release |

### Future Compatibility

When ABI v2 is released:
- Plugins built for ABI v1 will continue to work
- New features will only be available to v2 plugins
- Host will support both v1 and v2 simultaneously

---

## Best Practices

1. **Always check return codes** from host callbacks
2. **Copy message data** if needed beyond receive callback
3. **Use const-correct types** for callback parameters
4. **Handle errors gracefully** - don't panic
5. **Free resources** in `mef_component_cleanup`
6. **Validate configuration** in `mef_component_init`
7. **Use static strings** for `mef_component_info`
8. **Metric names should be lowercase** with underscores

---

## Reference Implementation

See the official plugin SDK for Rust:
- [`sunrush-plugin-sdk`](../crates/plugin-sdk/)
- [Example plugins](../plugins/)

---

**This ABI specification is stable and will be maintained for backward compatibility. 🔒**
