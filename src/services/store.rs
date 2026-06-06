use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

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
            baud_rate: 115200,
            data_bits: 8,
            stop_bits: "1".to_string(),
            parity: "none".to_string(),
            flow_control: "none".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Encoding {
    Ascii,
    Hex,
    Utf8,
    Gbk,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageDirection {
    Received,
    Sent,
}

fn default_message_direction() -> MessageDirection {
    MessageDirection::Received
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SendLineEnding {
    None,
    Cr,
    Lf,
    Crlf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivedMessage {
    #[serde(default = "default_message_direction")]
    pub direction: MessageDirection,
    pub timestamp: String,
    pub data: String,
    pub encoding: Encoding,
    pub raw_bytes: Vec<u8>,
}

#[derive(Debug, Clone, Deserialize)]
struct TauriEventPayload<T> {
    payload: T,
}

pub fn parse_received_message_event(value: serde_json::Value) -> Result<ReceivedMessage, String> {
    match serde_json::from_value::<TauriEventPayload<ReceivedMessage>>(value.clone()) {
        Ok(event) => Ok(event.payload),
        Err(payload_error) => {
            serde_json::from_value::<ReceivedMessage>(value).map_err(|direct_error| {
                format!(
                    "无法解析串口事件 payload: {}; 直接解析消息也失败: {}",
                    payload_error, direct_error
                )
            })
        }
    }
}

pub fn received_message_render_key(index: usize, message: &ReceivedMessage) -> String {
    format!("{}-{}", index, message.timestamp)
}

pub fn claim_serial_data_listener_registration(registered: &mut bool) -> bool {
    if *registered {
        false
    } else {
        *registered = true;
        true
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReceiveLineBuffer {
    pending: String,
    skip_next_lf: bool,
}

impl ReceiveLineBuffer {
    pub fn clear(&mut self) {
        self.pending.clear();
        self.skip_next_lf = false;
    }
}

pub fn split_received_message_lines(
    buffer: &mut ReceiveLineBuffer,
    message: ReceivedMessage,
) -> Vec<ReceivedMessage> {
    let mut lines = Vec::new();

    for ch in message.data.chars() {
        if buffer.skip_next_lf {
            buffer.skip_next_lf = false;
            if ch == '\n' {
                continue;
            }
        }

        match ch {
            '\r' => {
                lines.push(received_line_from_buffer(buffer, &message));
                buffer.skip_next_lf = true;
            }
            '\n' => {
                lines.push(received_line_from_buffer(buffer, &message));
            }
            _ => buffer.pending.push(ch),
        }
    }

    lines
}

fn received_line_from_buffer(
    buffer: &mut ReceiveLineBuffer,
    message: &ReceivedMessage,
) -> ReceivedMessage {
    ReceivedMessage {
        direction: MessageDirection::Received,
        timestamp: message.timestamp.clone(),
        data: std::mem::take(&mut buffer.pending),
        encoding: message.encoding.clone(),
        raw_bytes: Vec::new(),
    }
}

pub fn apply_send_line_ending(
    content: &str,
    encoding: &Encoding,
    line_ending: &SendLineEnding,
) -> String {
    match encoding {
        Encoding::Hex => apply_hex_line_ending(content, line_ending),
        Encoding::Ascii | Encoding::Utf8 | Encoding::Gbk => {
            format!("{}{}", content, text_line_ending(line_ending))
        }
    }
}

pub fn parse_send_line_ending_value(value: &str) -> SendLineEnding {
    match value {
        "cr" => SendLineEnding::Cr,
        "lf" => SendLineEnding::Lf,
        "crlf" => SendLineEnding::Crlf,
        _ => SendLineEnding::None,
    }
}

pub fn send_line_ending_value(line_ending: &SendLineEnding) -> &'static str {
    match line_ending {
        SendLineEnding::None => "none",
        SendLineEnding::Cr => "cr",
        SendLineEnding::Lf => "lf",
        SendLineEnding::Crlf => "crlf",
    }
}

fn apply_hex_line_ending(content: &str, line_ending: &SendLineEnding) -> String {
    let suffix = hex_line_ending(line_ending);
    if suffix.is_empty() {
        return content.to_string();
    }

    let content = content.trim_end();
    if content.is_empty() {
        suffix.to_string()
    } else {
        format!("{} {}", content, suffix)
    }
}

fn text_line_ending(line_ending: &SendLineEnding) -> &'static str {
    match line_ending {
        SendLineEnding::None => "",
        SendLineEnding::Cr => "\r",
        SendLineEnding::Lf => "\n",
        SendLineEnding::Crlf => "\r\n",
    }
}

fn hex_line_ending(line_ending: &SendLineEnding) -> &'static str {
    match line_ending {
        SendLineEnding::None => "",
        SendLineEnding::Cr => "0D",
        SendLineEnding::Lf => "0A",
        SendLineEnding::Crlf => "0D 0A",
    }
}

pub fn sent_message(timestamp: &str, data: String, encoding: Encoding) -> ReceivedMessage {
    ReceivedMessage {
        direction: MessageDirection::Sent,
        timestamp: timestamp.to_string(),
        data,
        encoding,
        raw_bytes: Vec::new(),
    }
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
            '\r' => "\\r".to_string(),
            '\n' => "\\n".to_string(),
            _ => ch.to_string(),
        })
        .collect()
}

pub fn current_message_timestamp() -> String {
    chrono::Local::now().format("%H:%M:%S%.3f").to_string()
}

pub fn should_continue_loop_send(is_connected: bool, loop_send: bool) -> bool {
    is_connected && loop_send
}

pub fn normalize_loop_interval_ms(interval_ms: u64) -> u64 {
    interval_ms.max(1)
}

pub fn can_toggle_connection(is_connected: bool, port_name: &str, is_in_progress: bool) -> bool {
    !is_in_progress && (is_connected || !port_name.is_empty())
}

#[cfg(test)]
mod loop_send_tests {
    use super::*;

    #[test]
    fn loop_send_stops_when_connection_is_closed() {
        assert!(!should_continue_loop_send(false, true));
        assert!(!should_continue_loop_send(true, false));
        assert!(should_continue_loop_send(true, true));
    }

    #[test]
    fn loop_interval_is_never_zero() {
        assert_eq!(normalize_loop_interval_ms(0), 1);
        assert_eq!(normalize_loop_interval_ms(250), 250);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub direction: String,
    pub encoding: String,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetCommand {
    pub id: String,
    pub name: String,
    pub content: String,
    pub encoding: Encoding,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActiveTab {
    ReceiveSend,
    CommandManager,
}

#[derive(Clone, Copy)]
pub struct AppState {
    pub is_connected: Signal<bool>,
    pub connection_in_progress: Signal<bool>,
    pub port_config: Signal<PortConfig>,
    pub active_tab: Signal<ActiveTab>,
    pub received_messages: Signal<Vec<ReceivedMessage>>,
    pub receive_line_buffer: Signal<ReceiveLineBuffer>,
    pub serial_data_listener_registered: Signal<bool>,
    pub log_entries: Signal<Vec<LogEntry>>,
    pub preset_commands: Signal<Vec<PresetCommand>>,
    pub bytes_received: Signal<u64>,
    pub bytes_sent: Signal<u64>,
    pub send_encoding: Signal<Encoding>,
    pub send_line_ending: Signal<SendLineEnding>,
    pub auto_scroll: Signal<bool>,
    pub loop_send: Signal<bool>,
    pub loop_interval_ms: Signal<u64>,
}

impl AppState {
    pub fn init() -> Self {
        Self {
            is_connected: Signal::new(false),
            connection_in_progress: Signal::new(false),
            port_config: Signal::new(PortConfig::default()),
            active_tab: Signal::new(ActiveTab::ReceiveSend),
            received_messages: Signal::new(Vec::new()),
            receive_line_buffer: Signal::new(ReceiveLineBuffer::default()),
            serial_data_listener_registered: Signal::new(false),
            log_entries: Signal::new(Vec::new()),
            preset_commands: Signal::new(Vec::new()),
            bytes_received: Signal::new(0),
            bytes_sent: Signal::new(0),
            send_encoding: Signal::new(Encoding::Ascii),
            send_line_ending: Signal::new(SendLineEnding::None),
            auto_scroll: Signal::new(true),
            loop_send: Signal::new(false),
            loop_interval_ms: Signal::new(1000),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_serial_data_from_tauri_event_payload() {
        let event = json!({
            "event": "serial-data",
            "id": 1,
            "payload": {
                "timestamp": "12:34:56.789",
                "data": "hello",
                "encoding": "ascii",
                "raw_bytes": [104, 101, 108, 108, 111]
            }
        });

        let message = parse_received_message_event(event).unwrap();

        assert_eq!(message.timestamp, "12:34:56.789");
        assert_eq!(message.data, "hello");
        assert_eq!(message.encoding, Encoding::Ascii);
        assert_eq!(message.direction, MessageDirection::Received);
        assert_eq!(message.raw_bytes, vec![104, 101, 108, 108, 111]);
    }

    #[test]
    fn apply_send_line_ending_appends_text_endings_for_text_encodings() {
        assert_eq!(
            apply_send_line_ending("AT", &Encoding::Ascii, &SendLineEnding::None),
            "AT"
        );
        assert_eq!(
            apply_send_line_ending("AT", &Encoding::Ascii, &SendLineEnding::Cr),
            "AT\r"
        );
        assert_eq!(
            apply_send_line_ending("AT", &Encoding::Utf8, &SendLineEnding::Lf),
            "AT\n"
        );
        assert_eq!(
            apply_send_line_ending("AT", &Encoding::Gbk, &SendLineEnding::Crlf),
            "AT\r\n"
        );
    }

    #[test]
    fn apply_send_line_ending_appends_hex_bytes_for_hex_encoding() {
        assert_eq!(
            apply_send_line_ending("41 54", &Encoding::Hex, &SendLineEnding::Cr),
            "41 54 0D"
        );
        assert_eq!(
            apply_send_line_ending("41 54", &Encoding::Hex, &SendLineEnding::Lf),
            "41 54 0A"
        );
        assert_eq!(
            apply_send_line_ending("41 54", &Encoding::Hex, &SendLineEnding::Crlf),
            "41 54 0D 0A"
        );
        assert_eq!(
            apply_send_line_ending("", &Encoding::Hex, &SendLineEnding::Crlf),
            "0D 0A"
        );
    }

    #[test]
    fn sent_message_marks_direction_and_visualizes_control_characters() {
        let message = sent_message("12:34:56.789", "AT\r\n".to_string(), Encoding::Ascii);

        assert_eq!(message.direction, MessageDirection::Sent);
        assert_eq!(message_direction_label(&message.direction), "发送");
        assert_eq!(visualize_serial_data(&message.data), "AT\\r\\n");
    }

    #[test]
    fn received_message_render_keys_stay_unique_when_timestamps_match() {
        let first = ReceivedMessage {
            direction: MessageDirection::Received,
            timestamp: "12:34:56.789".to_string(),
            data: "first".to_string(),
            encoding: Encoding::Ascii,
            raw_bytes: vec![1],
        };
        let second = ReceivedMessage {
            direction: MessageDirection::Received,
            timestamp: "12:34:56.789".to_string(),
            data: "second".to_string(),
            encoding: Encoding::Ascii,
            raw_bytes: vec![2],
        };

        assert_ne!(
            received_message_render_key(0, &first),
            received_message_render_key(1, &second)
        );
    }

    #[test]
    fn serial_data_listener_registration_can_only_be_claimed_once() {
        let mut registered = false;

        assert!(claim_serial_data_listener_registration(&mut registered));
        assert!(!claim_serial_data_listener_registration(&mut registered));
        assert!(registered);
    }

    #[test]
    fn connection_toggle_is_disabled_while_request_is_in_progress() {
        assert!(!can_toggle_connection(false, "tty.usbserial", true));
        assert!(!can_toggle_connection(true, "tty.usbserial", true));
        assert!(!can_toggle_connection(false, "", false));
        assert!(can_toggle_connection(false, "tty.usbserial", false));
        assert!(can_toggle_connection(true, "", false));
    }

    #[test]
    fn split_received_message_lines_uses_received_newline_characters() {
        let mut buffer = ReceiveLineBuffer::default();
        let message = received_message("first\r\nsecond\nthird\r");

        let lines = split_received_message_lines(&mut buffer, message);

        assert_eq!(
            lines
                .iter()
                .map(|line| line.data.as_str())
                .collect::<Vec<_>>(),
            vec!["first", "second", "third"]
        );
        assert!(buffer.pending.is_empty());
    }

    #[test]
    fn split_received_message_lines_buffers_incomplete_line_between_events() {
        let mut buffer = ReceiveLineBuffer::default();

        let first_lines = split_received_message_lines(&mut buffer, received_message("hello "));
        assert!(first_lines.is_empty());
        assert_eq!(buffer.pending, "hello ");

        let second_lines =
            split_received_message_lines(&mut buffer, received_message("world\nnext"));

        assert_eq!(
            second_lines
                .iter()
                .map(|line| line.data.as_str())
                .collect::<Vec<_>>(),
            vec!["hello world"]
        );
        assert_eq!(buffer.pending, "next");
    }

    #[test]
    fn split_received_message_lines_treats_crlf_across_events_as_one_break() {
        let mut buffer = ReceiveLineBuffer::default();

        let first_lines = split_received_message_lines(&mut buffer, received_message("hello\r"));
        let second_lines = split_received_message_lines(&mut buffer, received_message("\nnext\n"));

        assert_eq!(
            first_lines
                .iter()
                .map(|line| line.data.as_str())
                .collect::<Vec<_>>(),
            vec!["hello"]
        );
        assert_eq!(
            second_lines
                .iter()
                .map(|line| line.data.as_str())
                .collect::<Vec<_>>(),
            vec!["next"]
        );
        assert!(buffer.pending.is_empty());
    }

    fn received_message(data: &str) -> ReceivedMessage {
        ReceivedMessage {
            direction: MessageDirection::Received,
            timestamp: "12:34:56.789".to_string(),
            data: data.to_string(),
            encoding: Encoding::Ascii,
            raw_bytes: data.as_bytes().to_vec(),
        }
    }
}
