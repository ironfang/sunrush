// ---------------------------------------------------------------------------
// BusEvent — trait for typed messages
// ---------------------------------------------------------------------------

/// Implement this for any struct you want to send on the bus.
///
/// Default implementations of `encode` and `decode` use a plain bitwise
/// copy (`repr(C)` + `Copy` guarantees this is safe and deterministic).
/// Override them only if you need custom serialization.
pub trait BusEvent: Copy + Send + Sync + 'static {
    /// The topic this event type is always published on.
    const TOPIC: &'static str;

    /// Encode `self` into owned bytes.
    ///
    /// Default: reinterprets the value as raw bytes — zero allocation,
    /// zero copy.  Only correct for fully-initialized `#[repr(C)]` types
    /// with no padding bytes containing undefined values.
    fn encode(&self) -> Box<[u8]> {
        let bytes = unsafe {
            std::slice::from_raw_parts(
                self as *const Self as *const u8,
                std::mem::size_of::<Self>(),
            )
        };
        bytes.into()
    }

    /// Decode from bytes.
    ///
    /// Default: copies bytes into a new `Self` using an unaligned read.
    /// Returns `None` if the slice is shorter than `size_of::<Self>()`.
    fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < std::mem::size_of::<Self>() {
            return None;
        }
        // SAFETY: length checked above; read_unaligned handles any alignment.
        Some(unsafe { (bytes.as_ptr() as *const Self).read_unaligned() })
    }
}

// ---------------------------------------------------------------------------
// Messages
//
// Rules for every message type:
//   1. Must be `#[repr(C)]` so layout is deterministic across plugins.
//   2. Must be `Copy` — no heap-allocated fields (no String, Vec, Box, …).
//   3. `BusEvent` impl is a one-liner using the helpers above.
// ---------------------------------------------------------------------------

const MAX_NAME_LEN: usize = 63;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TestPayload {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub name_len: u8,
    /// UTF-8 name, zero-padded.  Use [`TestPayload::name()`] to read it.
    pub name: [u8; MAX_NAME_LEN],
}

impl TestPayload {
    pub fn new(id: u32, x: f32, y: f32, name: &str) -> Self {
        let bytes = name.as_bytes();
        let len = bytes.len().min(MAX_NAME_LEN) as u8;
        let mut buf = [0u8; MAX_NAME_LEN];
        buf[..len as usize].copy_from_slice(&bytes[..len as usize]);
        Self { id, x, y, name_len: len, name: buf }
    }

    pub fn name(&self) -> &str {
        std::str::from_utf8(&self.name[..self.name_len as usize])
            .unwrap_or("")
    }
}

impl BusEvent for TestPayload {
    const TOPIC: &'static str = "test.topic";
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payload_new_truncates_long_name() {
        let long = "a".repeat(100);
        let p = TestPayload::new(1, 0.0, 0.0, &long);
        assert_eq!(p.name_len as usize, MAX_NAME_LEN);
    }

    #[test]
    fn test_payload_short_name_is_aligned() {
        let short = "hello";
        let p = TestPayload::new(1, 0.0, 0.0, short);
        assert_eq!(p.name_len as usize, short.len());
        assert_eq!(p.name(), short);
        assert_eq!(p.name.len(), MAX_NAME_LEN);
    }

    #[test]
    fn bus_event_encode() {
        let event = TestPayload::new(42, 1.0, 2.0, "test");
        let encoded = event.encode();
        assert_eq!(encoded.len(), std::mem::size_of::<TestPayload>());
    }

    #[test]
    fn bus_event_encode_decode() {
        let event = TestPayload::new(42, 1.0, 2.0, "test");
        let encoded = event.encode();
        let decoded = TestPayload::decode(&encoded).unwrap();
        assert_eq!(decoded.id, event.id);
        assert_eq!(decoded.x, event.x);
        assert_eq!(decoded.y, event.y);
        assert_eq!(decoded.name_len, event.name_len);
        assert_eq!(decoded.name(), event.name());
    }
}