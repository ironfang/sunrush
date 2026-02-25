use sb::messages::BusEvent;
use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    future::Future,
    os::raw::{c_char, c_void},
    sync::{Mutex, OnceLock},
};

pub type BusCallback = extern "C" fn(topic: *const c_char, data: *const u8, len: usize);

#[repr(C)]
pub struct HostApi {
    pub host_ctx: *mut c_void,
    pub host_print: extern "C" fn(ctx: *mut c_void, msg: *const c_char),

    pub bus_ctx: *mut c_void,
    pub bus_publish:
        extern "C" fn(bus_ctx: *mut c_void, topic: *const c_char, data: *const u8, len: usize),

    pub bus_subscribe:
        extern "C" fn(bus_ctx: *mut c_void, topic: *const c_char, callback: BusCallback),
}

// Global registry: topic string → type-erased payload handler.
// Required because `BusCallback` is a bare `extern "C" fn` with no user-data
// pointer, so closures cannot be passed directly.
type PayloadHandler = Box<dyn Fn(&[u8]) + Send + Sync>;

fn handler_registry() -> &'static Mutex<HashMap<String, PayloadHandler>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, PayloadHandler>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Single trampoline that the host calls for every subscribed topic.
/// It looks up the registered handler by topic and invokes it.
extern "C" fn dispatch_trampoline(topic: *const c_char, data: *const u8, len: usize) {
    let topic_str = unsafe { CStr::from_ptr(topic) }.to_str().unwrap_or("");
    let payload = unsafe { std::slice::from_raw_parts(data, len) };
    if let Ok(registry) = handler_registry().lock() {
        if let Some(handler) = registry.get(topic_str) {
            handler(payload);
        }
    }
}

impl HostApi {
    pub fn publish<T>(&self, data: T)
    where
        T: BusEvent,
    {
        let encoded = data.encode();
        // Keep `c_topic` alive for the duration of the call.
        let c_topic = CString::new(T::TOPIC).expect("topic contains null byte");
        (self.bus_publish)(
            self.bus_ctx,
            c_topic.as_ptr(),
            encoded.as_ptr(),
            encoded.len(),
        );
    }

    /// Subscribe to bus events of type `T`.
    ///
    /// A single dedicated tokio task is spawned per topic.  The trampoline
    /// sends raw payload bytes onto an mpsc channel; the task drains it and
    /// calls the handler.  This avoids a `tokio::spawn` + `Arc::clone` on
    /// every received message.
    pub fn subscribe<T, F, Fut>(&self, handler: F)
    where
        T: BusEvent,
        F: Fn(T) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        // Unbounded so the sender (called synchronously from a tokio task)
        // never blocks.  Back-pressure comes from the broadcast channel.
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Box<[u8]>>();

        // One long-lived task per topic — drains the channel and calls handler.
        tokio::spawn(async move {
            while let Some(payload) = rx.recv().await {
                if let Some(event) = T::decode(&payload) {
                    handler(event).await;
                }
            }
        });

        // The trampoline just sends bytes — no Arc::clone, no spawn per msg.
        let erased: PayloadHandler = Box::new(move |payload: &[u8]| {
            let _ = tx.send(payload.into());
        });

        handler_registry()
            .lock()
            .expect("handler registry poisoned")
            .insert(T::TOPIC.to_string(), erased);

        // Keep `c_topic` alive across the FFI call.
        let c_topic = CString::new(T::TOPIC).expect("topic contains null byte");
        (self.bus_subscribe)(self.bus_ctx, c_topic.as_ptr(), dispatch_trampoline);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::RefCell,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Mutex,
        },
    };

    // -- Minimal BusEvent types with unique topics so parallel tests never
    //    collide in the global handler registry. --

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct PublishEvt {
        pub val: u32,
    }
    impl BusEvent for PublishEvt {
        const TOPIC: &'static str = "test.host_contract.publish";
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct SubscribeRegisterEvt {
        pub val: u32,
    }
    impl BusEvent for SubscribeRegisterEvt {
        const TOPIC: &'static str = "test.host_contract.subscribe_register";
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct SubscribeDeliverEvt {
        pub val: u64,
    }
    impl BusEvent for SubscribeDeliverEvt {
        const TOPIC: &'static str = "test.host_contract.subscribe_deliver";
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug)]
    struct ShortPayloadEvt {
        pub data: [u8; 32],
    }
    impl BusEvent for ShortPayloadEvt {
        const TOPIC: &'static str = "test.host_contract.short_payload";
    }

    // -- Thread-locals that mock FFI functions write into. --

    thread_local! {
        static PUBLISHED: RefCell<Option<(String, Vec<u8>)>> = const { RefCell::new(None) };
        static SUBSCRIBED_TOPIC: RefCell<Option<String>>     = const { RefCell::new(None) };
        static SUBSCRIBED_CB: RefCell<Option<BusCallback>>   = const { RefCell::new(None) };
    }

    extern "C" fn mock_print(_ctx: *mut c_void, _msg: *const c_char) {}

    extern "C" fn mock_publish(
        _ctx: *mut c_void,
        topic: *const c_char,
        data: *const u8,
        len: usize,
    ) {
        let t = unsafe { CStr::from_ptr(topic) }.to_str().unwrap().to_string();
        let d = unsafe { std::slice::from_raw_parts(data, len) }.to_vec();
        PUBLISHED.with(|p| *p.borrow_mut() = Some((t, d)));
    }

    extern "C" fn mock_subscribe(
        _ctx: *mut c_void,
        topic: *const c_char,
        cb: BusCallback,
    ) {
        let t = unsafe { CStr::from_ptr(topic) }.to_str().unwrap().to_string();
        SUBSCRIBED_TOPIC.with(|s| *s.borrow_mut() = Some(t));
        SUBSCRIBED_CB.with(|s| *s.borrow_mut() = Some(cb));
    }

    fn make_api() -> HostApi {
        HostApi {
            host_ctx: std::ptr::null_mut(),
            host_print: mock_print,
            bus_ctx: std::ptr::null_mut(),
            bus_publish: mock_publish,
            bus_subscribe: mock_subscribe,
        }
    }

    // -- Tests --

    /// `publish` forwards the correct topic string and an encodable payload
    /// to the underlying `bus_publish` FFI function.
    #[test]
    fn publish_sends_correct_topic_and_encoded_payload() {
        let api = make_api();
        api.publish(PublishEvt { val: 42 });

        PUBLISHED.with(|p| {
            let cap = p.borrow();
            let (topic, data) = cap.as_ref().expect("bus_publish was not called");
            assert_eq!(topic, PublishEvt::TOPIC);
            let decoded = PublishEvt::decode(data).expect("round-trip decode failed");
            assert_eq!(decoded.val, 42);
        });
    }

    /// `publish` encodes the full struct, not a subset of bytes.
    #[test]
    fn publish_encodes_full_struct_size() {
        let api = make_api();
        api.publish(PublishEvt { val: 0 });

        PUBLISHED.with(|p| {
            let cap = p.borrow();
            let (_, data) = cap.as_ref().unwrap();
            assert_eq!(data.len(), std::mem::size_of::<PublishEvt>());
        });
    }

    /// `subscribe` passes `dispatch_trampoline` (not a closure) and the
    /// correct topic string to `bus_subscribe`.
    #[test]
    fn subscribe_registers_trampoline_with_correct_topic() {
        let api = make_api();
        api.subscribe(|_e: SubscribeRegisterEvt| async {});

        SUBSCRIBED_TOPIC.with(|s| {
            let t = s.borrow();
            assert_eq!(t.as_deref(), Some(SubscribeRegisterEvt::TOPIC));
        });

        SUBSCRIBED_CB.with(|s| {
            let cb = s.borrow().expect("bus_subscribe was not called");
            assert_eq!(cb as usize, dispatch_trampoline as usize,
                "callback must be the global trampoline, not a per-call closure");
        });
    }

    /// End-to-end: subscribe → trampoline fires → handler decodes and runs.
    #[tokio::test]
    async fn subscribe_handler_receives_decoded_event() {
        let received: Arc<Mutex<Option<SubscribeDeliverEvt>>> = Arc::new(Mutex::new(None));
        let rx = Arc::clone(&received);

        let api = make_api();
        api.subscribe(move |e: SubscribeDeliverEvt| {
            let rx = Arc::clone(&rx);
            async move {
                *rx.lock().unwrap() = Some(e);
            }
        });

        // Simulate the host calling back with an encoded payload.
        let event = SubscribeDeliverEvt { val: 0xDEAD_BEEF_CAFE };
        let encoded = event.encode();
        let c_topic = CString::new(SubscribeDeliverEvt::TOPIC).unwrap();
        dispatch_trampoline(c_topic.as_ptr(), encoded.as_ptr(), encoded.len());

        // Yield so the spawned task runs on the current-thread runtime.
        tokio::task::yield_now().await;

        let got = received.lock().unwrap();
        let e = got.as_ref().expect("handler was never called");
        assert_eq!(e.val, 0xDEAD_BEEF_CAFE);
    }

    /// `dispatch_trampoline` must not panic when called for an unregistered topic.
    #[test]
    fn trampoline_ignores_unknown_topic() {
        let c_topic = CString::new("topic.nobody.subscribed.to").unwrap();
        let data = [0u8; 8];
        // Must not panic.
        dispatch_trampoline(c_topic.as_ptr(), data.as_ptr(), data.len());
    }

    /// When the payload is shorter than `size_of::<T>()`, `decode` returns
    /// `None` and the handler closure must never be invoked.
    #[test]
    fn trampoline_skips_handler_on_short_payload() {
        let called = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&called);

        let api = make_api();
        api.subscribe(move |_e: ShortPayloadEvt| {
            flag.store(true, Ordering::SeqCst);
            async {}
        });

        let c_topic = CString::new(ShortPayloadEvt::TOPIC).unwrap();
        let too_short = [0u8; 1]; // size_of::<ShortPayloadEvt>() == 32
        dispatch_trampoline(c_topic.as_ptr(), too_short.as_ptr(), too_short.len());

        assert!(!called.load(Ordering::SeqCst), "handler must not fire on truncated payload");
    }

    /// A second `subscribe` call for the same topic replaces the old handler.
    #[tokio::test]
    async fn subscribe_replaces_previous_handler_for_same_topic() {
        let first_called = Arc::new(AtomicBool::new(false));
        let second_called = Arc::new(AtomicBool::new(false));

        let f = Arc::clone(&first_called);
        let s = Arc::clone(&second_called);

        let api = make_api();

        api.subscribe(move |_e: SubscribeDeliverEvt| {
            f.store(true, Ordering::SeqCst);
            async {}
        });

        // Register a second handler — should overwrite the first in the registry.
        api.subscribe(move |_e: SubscribeDeliverEvt| {
            s.store(true, Ordering::SeqCst);
            async {}
        });

        let event = SubscribeDeliverEvt { val: 1 };
        let encoded = event.encode();
        let c_topic = CString::new(SubscribeDeliverEvt::TOPIC).unwrap();
        dispatch_trampoline(c_topic.as_ptr(), encoded.as_ptr(), encoded.len());

        tokio::task::yield_now().await;

        assert!(!first_called.load(Ordering::SeqCst),  "stale first handler must not fire");
        assert!(second_called.load(Ordering::SeqCst),  "new handler must fire");
    }
}
