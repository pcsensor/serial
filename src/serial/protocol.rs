use crate::state::Encoding;

#[derive(Debug, Clone)]
pub struct ReceivedData {
    pub timestamp: String,
    pub raw_bytes: Vec<u8>,
    pub encoding: Encoding,
}

#[derive(Debug, Clone)]
pub enum SerialEvent {
    Data(ReceivedData),
    Error(String),
    Disconnected,
}
