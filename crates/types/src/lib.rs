use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Message types that flow through the SunRush bus
#[derive(Debug, Clone)]
pub enum BusMessage {
    Shred(Shred),
    SlotEntry(SlotEntry),
    Block(Block),
    Transaction(MefTransaction),
}

/// Raw shred data from Jito ShredStream
#[derive(Debug, Clone)]
pub struct Shred {
    pub slot: u64,
    pub index: u32,
    pub data: Arc<Bytes>,
    pub receive_time: Option<Instant>,
}

/// A decodable entry within a slot
#[derive(Debug, Clone)]
pub struct SlotEntry {
    pub slot: u64,
    pub entry_index: u32,
    pub data: Arc<Bytes>,
}

/// Complete assembled block
#[derive(Debug, Clone)]
pub struct Block {
    pub slot: u64,
    pub parent_slot: u64,
    pub raw: Arc<Bytes>,
    pub blockhash: String,
}

/// Extracted transaction
#[derive(Debug, Clone)]
pub struct MefTransaction {
    pub slot: u64,
    pub signature: String,
    pub accounts: Vec<String>,
    pub instructions: Vec<TransactionInstruction>,
    pub raw_message: Arc<Bytes>,
    pub timestamp_ns: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionInstruction {
    pub program_id_index: u8,
    pub accounts: Vec<u8>,
    pub data: Vec<u8>,
}

/// Message type discriminator for ABI
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Shred = 0,
    SlotEntry = 1,
    Block = 2,
    Transaction = 3,
}

impl MessageType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(MessageType::Shred),
            1 => Some(MessageType::SlotEntry),
            2 => Some(MessageType::Block),
            3 => Some(MessageType::Transaction),
            _ => None,
        }
    }
}
