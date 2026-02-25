use std::{
    collections::HashMap,
    future::Future,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;

use crate::messages::BusEvent;

pub mod messages;

const CHANNEL_CAPACITY: usize = 256_000;

// ---------------------------------------------------------------------------
// BusMessage
// ---------------------------------------------------------------------------

/// A message travelling on the bus.
///
/// The bus takes ownership once and wraps it in an `Arc`.  Subscribers
/// receive an `Arc<BusMessage>` — a single atomic increment per subscriber,
/// no data copies.  The message is freed automatically when the last
/// subscriber handler drops its `Arc`.
#[derive(Debug)]
pub struct BusMessage {
    /// The topic this message was published on.
    ///
    /// Topics are expected to be a small set of compile-time constants, so a
    /// plain `&'static str` — a pointer + length into the binary — is the
    /// cheapest possible representation: no heap allocation, no ref-counting.
    pub topic: &'static str,
    /// The raw payload bytes.
    pub data: Box<[u8]>,
}

// ---------------------------------------------------------------------------
// Bus
// ---------------------------------------------------------------------------

struct TopicChannel {
    sender: broadcast::Sender<Arc<BusMessage>>,
}

/// The service bus.
///
/// Cheap to clone — all state lives behind an `Arc`.  Plugins should share
/// one `Bus` instance and interact through [`Publisher`] handles and
/// subscriber tasks.
#[derive(Clone, Default)]
pub struct Bus {
    topics: Arc<Mutex<HashMap<&'static str, TopicChannel>>>,
}

impl Bus {
    /// Create a new, empty bus.
    pub fn new() -> Self {
        Bus::default()
    }

    /// Return a [`Publisher`] for the given `topic`.
    ///
    /// If the topic's channel does not exist yet it is created now.
    /// Multiple calls with the same topic name return publishers that all
    /// share the same underlying broadcast channel.
    pub fn publisher(&self, topic: &'static str) -> Publisher {
        let sender = self.get_or_create(topic);
        Publisher { sender, topic }
    }

    /// Subscribe to `topic` with an async handler.
    ///
    /// A dedicated tokio task is spawned for this subscriber, so the handler
    /// never blocks the publisher or any other subscriber.  If the subscriber
    /// falls behind the channel capacity it will log a warning and continue
    /// from the newest available message.
    pub fn subscribe<F, Fut>(&self, topic: &'static str, handler: F)
    where
        F: Fn(Arc<BusMessage>) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let sender = self.get_or_create(topic);
        let mut rx = sender.subscribe();

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(msg) => {
                        handler(msg).await;
                    }
                    Err(broadcast::error::RecvError::Lagged(dropped)) => {
                        eprintln!(
                            "[sb] subscriber on topic '{}' lagged — {} messages dropped",
                            topic, dropped
                        );
                        // Keep going from the newest available message.
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        // All publishers for this topic have been dropped.
                        break;
                    }
                }
            }
        });
    }

    // --- internal helpers ---------------------------------------------------

    fn get_or_create(&self, topic: &'static str) -> broadcast::Sender<Arc<BusMessage>> {
        let mut map = self.topics.lock().unwrap();
        map.entry(topic)
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel(CHANNEL_CAPACITY);
                TopicChannel { sender: tx }
            })
            .sender
            .clone()
    }
}

// ---------------------------------------------------------------------------
// TypedPublisher
// ---------------------------------------------------------------------------

/// A strongly-typed publish handle for events of type `T: BusEvent`.
///
/// Obtained via [`Bus::typed_publisher`].  Encodes `T` and forwards
/// the bytes to the underlying [`Publisher`].
pub struct TypedPublisher<T: BusEvent> {
    inner: Publisher,
    _marker: std::marker::PhantomData<fn(T)>,
}

impl<T: BusEvent> TypedPublisher<T> {
    /// Encode and publish `event`.  Returns the subscriber count.
    pub fn publish(&self, event: &T) -> usize {
        self.inner.publish(event.encode())
    }
}

// ---------------------------------------------------------------------------
// Publisher
// ---------------------------------------------------------------------------

/// A handle for publishing messages on one specific topic.
///
/// Obtained via [`Bus::publisher`].  Cheap to clone.
#[derive(Clone)]
pub struct Publisher {
    sender: broadcast::Sender<Arc<BusMessage>>,
    topic: &'static str,
}

impl Publisher {
    /// Publish `data` to every active subscriber of this topic.
    ///
    /// Returns the number of subscribers that received the message.
    /// Returns `0` when there are no subscribers — this is not an error.
    ///
    /// Accepts any type that converts into `Box<[u8]>` without an extra
    /// allocation: `Box<[u8]>` (zero-cost move), `Vec<u8>` (moves the
    /// underlying allocation, reallocates only when `len < capacity`).
    /// Passing a `&[u8]` requires a copy — that is unavoidable because
    /// ownership must be transferred to the message.
    pub fn publish(&self, data: impl Into<Box<[u8]>>) -> usize {
        let msg = Arc::new(BusMessage {
            topic: self.topic,
            data: data.into(),
        });
        self.sender.send(msg).unwrap_or(0)
    }

    /// The topic name this publisher sends on.
    pub fn topic(&self) -> &'static str {
        self.topic
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn single_subscriber_receives_message() {
        let bus = Bus::new();
        let pub1 = bus.publisher("events");

        let counter = Arc::new(AtomicUsize::new(0));
        let c = Arc::clone(&counter);

        bus.subscribe("events", move |msg: Arc<BusMessage>| {
            let c = Arc::clone(&c);
            async move {
                assert_eq!(&*msg.data, b"hello");
                c.fetch_add(1, Ordering::SeqCst);
            }
        });

        // Yield so the subscriber task is ready before we publish.
        tokio::task::yield_now().await;
        pub1.publish(b"hello".to_vec());

        sleep(Duration::from_millis(50)).await;
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn multiple_subscribers_all_receive() {
        let bus = Bus::new();
        let publisher = bus.publisher("topic");

        let hits = Arc::new(AtomicUsize::new(0));

        for _ in 0..3 {
            let h = Arc::clone(&hits);
            bus.subscribe("topic", move |_msg: Arc<BusMessage>| {
                let h = Arc::clone(&h);
                async move {
                    h.fetch_add(1, Ordering::SeqCst);
                }
            });
        }

        tokio::task::yield_now().await;
        publisher.publish(b"ping".to_vec());

        sleep(Duration::from_millis(50)).await;
        assert_eq!(hits.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn publish_with_no_subscribers_is_fine() {
        let bus = Bus::new();
        let publisher = bus.publisher("empty");
        let sent = publisher.publish(b"noop".to_vec());
        assert_eq!(sent, 0);
    }

    #[tokio::test]
    async fn multiple_messages_are_all_delivered() {
        let bus = Bus::new();
        let publisher = bus.publisher("stream");

        let counter = Arc::new(AtomicUsize::new(0));
        let c = Arc::clone(&counter);

        bus.subscribe("stream", move |_msg: Arc<BusMessage>| {
            let c = Arc::clone(&c);
            async move {
                c.fetch_add(1, Ordering::SeqCst);
            }
        });

        tokio::task::yield_now().await;

        for _ in 0..10 {
            publisher.publish(b"msg".to_vec());
        }

        sleep(Duration::from_millis(50)).await;
        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }
}
