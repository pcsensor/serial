//! Application state and pure domain helpers.
//!
//! Keeping these types independent from GPUI makes the serial protocol easy to
//! test and prevents view code from becoming the source of truth for behavior.

use chrono::Timelike;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PortConfig {
    pub port_name: String,
    pub baud_rate: u32,
    pub data_bits: u8,
    pub stop_bits: String,
    pub parity: String,
    pub flow_control: String,
}

impl Default for PortConfig {
    fn default() -> Self {
        Self {
            port_name: String::new(),
            baud_rate: 115_200,
            data_bits: 8,
            stop_bits: "1".into(),
            parity: "none".into(),
            flow_control: "none".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Encoding {
    #[default]
    Ascii,
    Hex,
    Utf8,
    Gbk,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageDirection {
    Received,
    Sent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SendLineEnding {
    #[default]
    None,
    Cr,
    Lf,
    Crlf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivedMessage {
    #[serde(default = "default_direction")]
    pub direction: MessageDirection,
    pub timestamp: String,
    pub data: String,
    pub encoding: Encoding,
    #[serde(default)]
    pub raw_bytes: Vec<u8>,
}

fn default_direction() -> MessageDirection {
    MessageDirection::Received
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReceiveLineBuffer {
    pending: String,
    raw_pending: Vec<u8>,
    skip_next_lf: bool,
}

/// Maximum bytes retained for a line that has not received a CR/LF yet. A
/// device streaming binary data without line endings is segmented instead of
/// growing memory without bound.
pub const MAX_PENDING_RECEIVE_BYTES: usize = 64 * 1024;

impl ReceiveLineBuffer {
    pub fn clear(&mut self) {
        self.pending.clear();
        self.raw_pending.clear();
        self.skip_next_lf = false;
    }
    #[allow(dead_code)]
    pub fn pending(&self) -> &str {
        &self.pending
    }
    pub fn pending_bytes(&self) -> &[u8] {
        &self.raw_pending
    }
    pub fn has_pending(&self) -> bool {
        !self.raw_pending.is_empty()
    }
    pub fn take_pending(&mut self) -> Vec<u8> {
        self.pending.clear();
        self.skip_next_lf = false;
        std::mem::take(&mut self.raw_pending)
    }

    fn append(&mut self, byte: u8) {
        self.raw_pending.push(byte);
    }

    fn take(&mut self, encoding: &Encoding) -> (String, Vec<u8>) {
        let raw = std::mem::take(&mut self.raw_pending);
        let text = decode_received_bytes_lossy(&raw, encoding);
        self.pending.clear();
        (text, raw)
    }
}

/// Decode received bytes for display, retaining raw bytes separately for HEX
/// mode and exports. Lossy conversion keeps malformed or incomplete input
/// visible instead of dropping it or poisoning the stream buffer.
pub fn decode_received_bytes_lossy(bytes: &[u8], encoding: &Encoding) -> String {
    match encoding {
        Encoding::Ascii => bytes
            .iter()
            .map(|byte| {
                if *byte <= 0x7f {
                    *byte as char
                } else {
                    '\u{FFFD}'
                }
            })
            .collect(),
        Encoding::Hex => format_hex_bytes(bytes),
        Encoding::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
        Encoding::Gbk => encoding_rs::GBK.decode(bytes).0.into_owned(),
    }
}

/// Split raw serial input on CR/LF boundaries while preserving the exact
/// bytes for each complete message. CRLF spanning two reads is treated as one
/// delimiter; an incomplete trailing line remains in `buffer`.
pub fn split_received_data_lines(
    buffer: &mut ReceiveLineBuffer,
    timestamp: &str,
    raw_bytes: &[u8],
    encoding: &Encoding,
) -> Vec<ReceivedMessage> {
    let mut lines = Vec::new();
    for &byte in raw_bytes {
        if buffer.skip_next_lf {
            buffer.skip_next_lf = false;
            if byte == b'\n' {
                continue;
            }
        }
        match byte {
            b'\r' => {
                let (data, raw_bytes) = buffer.take(encoding);
                lines.push(ReceivedMessage {
                    direction: MessageDirection::Received,
                    timestamp: timestamp.to_owned(),
                    data,
                    encoding: encoding.clone(),
                    raw_bytes,
                });
                buffer.skip_next_lf = true;
            }
            b'\n' => {
                let (data, raw_bytes) = buffer.take(encoding);
                lines.push(ReceivedMessage {
                    direction: MessageDirection::Received,
                    timestamp: timestamp.to_owned(),
                    data,
                    encoding: encoding.clone(),
                    raw_bytes,
                });
            }
            byte => {
                buffer.append(byte);
                if buffer.raw_pending.len() >= MAX_PENDING_RECEIVE_BYTES {
                    let (data, raw_bytes) = buffer.take(encoding);
                    lines.push(ReceivedMessage {
                        direction: MessageDirection::Received,
                        timestamp: timestamp.to_owned(),
                        data,
                        encoding: encoding.clone(),
                        raw_bytes,
                    });
                }
            }
        }
    }
    buffer.pending = decode_received_bytes_lossy(&buffer.raw_pending, encoding);
    lines
}

/// Split a stream into lines while retaining incomplete data for the next read.
#[allow(dead_code)]
pub fn split_received_message_lines(
    buffer: &mut ReceiveLineBuffer,
    message: ReceivedMessage,
) -> Vec<ReceivedMessage> {
    let bytes = if message.raw_bytes.is_empty() {
        message.data.as_bytes().to_vec()
    } else {
        message.raw_bytes
    };
    split_received_data_lines(buffer, &message.timestamp, &bytes, &message.encoding)
}

pub fn apply_send_line_ending(
    content: &str,
    encoding: &Encoding,
    ending: &SendLineEnding,
) -> String {
    let suffix = match (encoding, ending) {
        (Encoding::Hex, SendLineEnding::None) => "",
        (Encoding::Hex, SendLineEnding::Cr) => "0D",
        (Encoding::Hex, SendLineEnding::Lf) => "0A",
        (Encoding::Hex, SendLineEnding::Crlf) => "0D 0A",
        (_, SendLineEnding::None) => "",
        (_, SendLineEnding::Cr) => "\r",
        (_, SendLineEnding::Lf) => "\n",
        (_, SendLineEnding::Crlf) => "\r\n",
    };
    if suffix.is_empty() {
        return content.to_owned();
    }
    if matches!(encoding, Encoding::Hex) {
        let clean = content.trim_end();
        if clean.is_empty() {
            suffix.to_owned()
        } else {
            format!("{clean} {suffix}")
        }
    } else {
        format!("{content}{suffix}")
    }
}

pub fn sent_message(
    timestamp: &str,
    data: String,
    encoding: Encoding,
    raw_bytes: Vec<u8>,
) -> ReceivedMessage {
    ReceivedMessage {
        direction: MessageDirection::Sent,
        timestamp: timestamp.into(),
        data,
        encoding,
        raw_bytes,
    }
}

pub fn current_message_timestamp() -> String {
    let now = chrono::Local::now();
    format!(
        "{:02}:{:02}:{:02}.{:03}",
        now.hour(),
        now.minute(),
        now.second(),
        now.nanosecond() / 1_000_000
    )
}

pub fn message_direction_label(direction: &MessageDirection) -> &'static str {
    match direction {
        MessageDirection::Received => "收到",
        MessageDirection::Sent => "发送",
    }
}

pub fn visualize_serial_data(data: &str) -> String {
    data.chars()
        .map(|ch| match ch {
            '\r' => "\\r".into(),
            '\n' => "\\n".into(),
            _ => ch.to_string(),
        })
        .collect()
}

pub fn format_message_display(msg: &ReceivedMessage, hex_display: bool) -> String {
    if !hex_display {
        return visualize_serial_data(&msg.data);
    }
    if !msg.raw_bytes.is_empty() {
        return format_hex_bytes(&msg.raw_bytes);
    }
    match msg.encoding {
        Encoding::Hex => msg
            .data
            .split_whitespace()
            .map(|b| b.to_uppercase())
            .collect::<Vec<_>>()
            .join(" "),
        _ => format_hex_bytes(msg.data.as_bytes()),
    }
}

pub fn format_hex_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn new_preset_command_id() -> String {
    Uuid::new_v4().to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetCommand {
    pub id: String,
    pub name: String,
    pub content: String,
    pub encoding: Encoding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub direction: String,
    pub encoding: String,
    pub data: String,
}

pub const MAX_LOG_ENTRIES: usize = 10_000;

pub fn push_log_bounded(log: &mut Vec<ReceivedMessage>, message: ReceivedMessage) {
    log.push(message);
    if log.len() > MAX_LOG_ENTRIES {
        let excess = log.len() - MAX_LOG_ENTRIES;
        log.drain(0..excess);
    }
}

pub fn normalize_loop_interval_ms(value: u64) -> u64 {
    value.clamp(1, 86_400_000)
}
pub fn should_continue_loop_send(
    is_connected: bool,
    enabled: bool,
    generation: u64,
    current_generation: u64,
) -> bool {
    is_connected && enabled && generation == current_generation
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    ReceiveSend,
    CommandManager,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub connected: bool,
    pub connecting: bool,
    pub config: PortConfig,
    pub active_tab: ActiveTab,
    pub messages: Vec<ReceivedMessage>,
    pub receive_buffer: ReceiveLineBuffer,
    pub receive_preview: String,
    pub bytes_received: u64,
    pub bytes_sent: u64,
    pub send_encoding: Encoding,
    pub send_line_ending: SendLineEnding,
    pub auto_scroll: bool,
    pub loop_send: bool,
    pub loop_interval_ms: u64,
    pub hex_display: bool,
    pub status: String,
    pub error: Option<String>,
    pub presets: Vec<PresetCommand>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            connected: false,
            connecting: false,
            config: PortConfig::default(),
            active_tab: ActiveTab::ReceiveSend,
            messages: Vec::new(),
            receive_buffer: ReceiveLineBuffer::default(),
            receive_preview: String::new(),
            bytes_received: 0,
            bytes_sent: 0,
            send_encoding: Encoding::Ascii,
            send_line_ending: SendLineEnding::None,
            auto_scroll: true,
            loop_send: false,
            loop_interval_ms: 1000,
            hex_display: false,
            status: "未连接".into(),
            error: None,
            presets: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn split_keeps_partial_lines() {
        let mut b = ReceiveLineBuffer::default();
        assert!(split_received_data_lines(&mut b, "t", b"hello", &Encoding::Ascii).is_empty());
        assert_eq!(b.pending(), "hello");
    }
    #[test]
    fn crlf_is_single_break() {
        let mut b = ReceiveLineBuffer::default();
        let out = split_received_data_lines(&mut b, "t", b"a\r\nb\n", &Encoding::Ascii);
        assert_eq!(
            out.iter().map(|m| m.data.as_str()).collect::<Vec<_>>(),
            ["a", "b"]
        );
        assert_eq!(out[0].raw_bytes, b"a");
    }

    #[test]
    fn raw_hex_input_is_framed_before_display_decoding() {
        let mut b = ReceiveLineBuffer::default();
        let out = split_received_data_lines(&mut b, "t", &[0x41, 0x0d, 0x0a], &Encoding::Hex);
        assert_eq!(out[0].data, "41");
        assert_eq!(out[0].raw_bytes, vec![0x41]);
    }

    #[test]
    fn crlf_split_across_reads_keeps_following_data() {
        let mut b = ReceiveLineBuffer::default();
        assert_eq!(
            split_received_data_lines(&mut b, "t", b"first\r", &Encoding::Utf8)
                .pop()
                .unwrap()
                .data,
            "first"
        );
        let out = split_received_data_lines(&mut b, "t", b"\nsecond\n", &Encoding::Utf8);
        assert_eq!(
            out.iter().map(|m| m.data.as_str()).collect::<Vec<_>>(),
            ["second"]
        );
    }

    #[test]
    fn unterminated_input_is_segmented_at_the_pending_limit() {
        let mut b = ReceiveLineBuffer::default();
        let out = split_received_data_lines(
            &mut b,
            "t",
            &vec![b'x'; MAX_PENDING_RECEIVE_BYTES + 1],
            &Encoding::Ascii,
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].raw_bytes.len(), MAX_PENDING_RECEIVE_BYTES);
        assert_eq!(b.pending_bytes().len(), 1);
    }
    #[test]
    fn hex_ending_is_valid() {
        assert_eq!(
            apply_send_line_ending("41", &Encoding::Hex, &SendLineEnding::Crlf),
            "41 0D 0A"
        );
    }
    #[test]
    fn bounded_log_applies_to_sent_and_received() {
        let mut log = Vec::new();
        for _ in 0..(MAX_LOG_ENTRIES + 1) {
            push_log_bounded(
                &mut log,
                ReceivedMessage {
                    direction: MessageDirection::Received,
                    timestamp: "t".into(),
                    data: "x".into(),
                    encoding: Encoding::Ascii,
                    raw_bytes: vec![],
                },
            );
        }
        assert_eq!(log.len(), MAX_LOG_ENTRIES);
    }
    #[test]
    fn loop_generation_invalidates_old_worker() {
        assert!(!should_continue_loop_send(true, true, 1, 2));
        assert!(should_continue_loop_send(true, true, 2, 2));
    }
}
