use std::collections::HashMap;
use std::ffi::{CStr, CString, c_void, c_char};
use std::sync::{Arc, Mutex};
use sb::{Bus, BusMessage};
use abi::host_contract::{BusCallback, HostApi};

fn intern_topic(s: &str) -> &'static str {
    static TABLE: Mutex<Option<HashMap<&'static str, &'static str>>> = Mutex::new(None);
    let mut guard = TABLE.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    if let Some(&existing) = map.get(s) {
        return existing;
    }
    let leaked: &'static str = Box::leak(s.to_owned().into_boxed_str());
    map.insert(leaked, leaked);
    leaked
}

extern "C" fn host_print(_ctx: *mut c_void, msg: *const c_char) {
    let s = unsafe { CStr::from_ptr(msg) }.to_string_lossy();
    println!("[plugin] {}", s);
}

extern "C" fn bus_publish(
    bus_ctx: *mut c_void,
    topic: *const c_char,
    data: *const u8,
    len: usize,
) {
    let bus = unsafe { &*(bus_ctx as *const Bus) };
    let topic_str = unsafe { CStr::from_ptr(topic) }.to_str().unwrap_or("");
    let topic_static = intern_topic(topic_str);
    let payload = unsafe { std::slice::from_raw_parts(data, len) }.to_vec();
    bus.publisher(topic_static).publish(payload);
}

extern "C" fn bus_subscribe(
    bus_ctx: *mut c_void,
    topic: *const c_char,
    callback: BusCallback,
) {
    let bus = unsafe { &*(bus_ctx as *const Bus) };
    let topic_static = intern_topic(
        unsafe { CStr::from_ptr(topic) }.to_str().unwrap_or("")
    );

    // Leak a CString for this topic so the pointer is valid for 'static.
    // Subscriptions are permanent, so this tiny allocation is intentional.
    let topic_cstr: &'static std::ffi::CStr =
        Box::leak(CString::new(topic_static).expect("topic has null byte").into_boxed_c_str());

    bus.subscribe(topic_static, move |msg: Arc<BusMessage>| {
        async move {
            callback(topic_cstr.as_ptr(), msg.data.as_ptr(), msg.data.len());
        }
    });
}

pub fn init() -> HostApi {
    let bus = Box::new(Bus::new());
    let bus_ptr = Box::into_raw(bus);

    HostApi {
        host_ctx: std::ptr::null_mut(),
        host_print,
        bus_ctx: bus_ptr as *mut c_void,
        bus_publish,
        bus_subscribe,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_topic_content_matches_input() {
        assert_eq!(intern_topic("foo.bar"), "foo.bar");
    }

    #[test]
    fn intern_topic_same_input_returns_same_pointer() {
        let a = intern_topic("same.topic");
        let b = intern_topic("same.topic");
        assert!(std::ptr::eq(a, b), "expected same pointer for same input");
    }

    #[test]
    fn intern_topic_different_inputs_return_different_pointers() {
        let a = intern_topic("topic.alpha");
        let b = intern_topic("topic.beta");
        assert!(!std::ptr::eq(a, b));
    }

    #[test]
    fn intern_topic_empty_string() {
        let s = intern_topic("");
        assert_eq!(s, "");
    }
}
