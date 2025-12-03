use sunrush_types::{Block, BusMessage, MefTransaction, Shred, SlotEntry};
use tokio::sync::broadcast;
use tracing::{debug, warn};

/// In-process message bus using Tokio broadcast channels
pub struct MessageBus {
    shred_tx: broadcast::Sender<Shred>,
    entry_tx: broadcast::Sender<SlotEntry>,
    block_tx: broadcast::Sender<Block>,
    tx_tx: broadcast::Sender<MefTransaction>,
}

impl MessageBus {
    /// Create a new message bus with specified capacity per channel
    pub fn new(capacity: usize) -> Self {
        Self {
            shred_tx: broadcast::channel(capacity).0,
            entry_tx: broadcast::channel(capacity).0,
            block_tx: broadcast::channel(capacity).0,
            tx_tx: broadcast::channel(capacity).0,
        }
    }

    /// Publish a message to the appropriate channel
    pub fn publish(&self, msg: BusMessage) -> Result<usize, BusError> {
        match msg {
            BusMessage::Shred(s) => self
                .shred_tx
                .send(s)
                .map_err(|_| BusError::NoReceivers),
            BusMessage::SlotEntry(e) => self
                .entry_tx
                .send(e)
                .map_err(|_| BusError::NoReceivers),
            BusMessage::Block(b) => self
                .block_tx
                .send(b)
                .map_err(|_| BusError::NoReceivers),
            BusMessage::Transaction(t) => self
                .tx_tx
                .send(t)
                .map_err(|_| BusError::NoReceivers),
        }
    }

    /// Subscribe to shred messages
    pub fn subscribe_shreds(&self) -> broadcast::Receiver<Shred> {
        self.shred_tx.subscribe()
    }

    /// Subscribe to slot entry messages
    pub fn subscribe_entries(&self) -> broadcast::Receiver<SlotEntry> {
        self.entry_tx.subscribe()
    }

    /// Subscribe to block messages
    pub fn subscribe_blocks(&self) -> broadcast::Receiver<Block> {
        self.block_tx.subscribe()
    }

    /// Subscribe to transaction messages
    pub fn subscribe_transactions(&self) -> broadcast::Receiver<MefTransaction> {
        self.tx_tx.subscribe()
    }

    /// Get the number of active receivers for each channel
    pub fn receiver_count(&self) -> BusStats {
        BusStats {
            shred_receivers: self.shred_tx.receiver_count(),
            entry_receivers: self.entry_tx.receiver_count(),
            block_receivers: self.block_tx.receiver_count(),
            tx_receivers: self.tx_tx.receiver_count(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BusStats {
    pub shred_receivers: usize,
    pub entry_receivers: usize,
    pub block_receivers: usize,
    pub tx_receivers: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum BusError {
    #[error("No receivers for message")]
    NoReceivers,
    #[error("Channel lagged by {0} messages")]
    Lagged(u64),
    #[error("Channel closed")]
    Closed,
}

impl From<broadcast::error::RecvError> for BusError {
    fn from(err: broadcast::error::RecvError) -> Self {
        match err {
            broadcast::error::RecvError::Lagged(n) => BusError::Lagged(n),
            broadcast::error::RecvError::Closed => BusError::Closed,
        }
    }
}
