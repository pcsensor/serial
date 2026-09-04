use crate::state::Encoding;

pub fn encode(input: &str, encoding: &Encoding) -> Result<Vec<u8>, String> {
    match encoding {
        Encoding::Ascii => {
            if input.bytes().any(|b| b > 0x7f) {
                return Err("ASCII 只能包含 0-127 范围内的字符".into());
            }
            Ok(input.as_bytes().to_vec())
        }
        Encoding::Utf8 => Ok(input.as_bytes().to_vec()),
        Encoding::Hex => parse_hex(input),
        Encoding::Gbk => {
            let (bytes, _, had_errors) = encoding_rs::GBK.encode(input);
            if had_errors {
                Err("GBK 编码失败：包含不支持的字符".into())
            } else {
                Ok(bytes.into_owned())
            }
        }
    }
}

pub fn decode(data: &[u8], encoding: &Encoding) -> Result<String, String> {
    match encoding {
        Encoding::Ascii => {
            if data.iter().any(|b| *b > 0x7f) {
                return Err("收到非 ASCII 字节".into());
            }
            Ok(data.iter().map(|b| *b as char).collect())
        }
        Encoding::Utf8 => {
            String::from_utf8(data.to_vec()).map_err(|e| format!("UTF-8 解码失败: {e}"))
        }
        Encoding::Hex => Ok(data
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ")),
        Encoding::Gbk => {
            let (text, _, had_errors) = encoding_rs::GBK.decode(data);
            if had_errors {
                Err("GBK 解码失败：无效的字节序列".into())
            } else {
                Ok(text.into_owned())
            }
        }
    }
}

fn parse_hex(input: &str) -> Result<Vec<u8>, String> {
    let cleaned: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    if !cleaned.len().is_multiple_of(2) {
        return Err("HEX 字符串长度必须为偶数".into());
    }
    (0..cleaned.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&cleaned[i..i + 2], 16)
                .map_err(|_| format!("无效的 HEX 字符: {}", &cleaned[i..i + 2]))
        })
        .collect()
}

/// Incremental decoder used by the receive path. Incomplete UTF-8 tails are
/// retained instead of being rendered as a spurious replacement/error.
#[derive(Debug, Default)]
pub struct StreamDecoder {
    pending: Vec<u8>,
}

impl StreamDecoder {
    pub fn clear(&mut self) {
        self.pending.clear();
    }
    pub fn decode(&mut self, bytes: &[u8], encoding: &Encoding) -> Result<String, String> {
        if !matches!(encoding, Encoding::Utf8) {
            return decode(bytes, encoding);
        }
        self.pending.extend_from_slice(bytes);
        match String::from_utf8(std::mem::take(&mut self.pending)) {
            Ok(text) => Ok(text),
            Err(error) => {
                let valid = error.utf8_error().valid_up_to();
                let bytes = error.into_bytes();
                self.pending.extend_from_slice(&bytes[valid..]);
                Ok(String::from_utf8_lossy(&bytes[..valid]).into_owned())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ascii_rejects_non_ascii() {
        assert!(encode("你好", &Encoding::Ascii).is_err());
    }
    #[test]
    fn hex_roundtrip() {
        let b = encode("48 65", &Encoding::Hex).unwrap();
        assert_eq!(b, [0x48, 0x65]);
        assert_eq!(decode(&b, &Encoding::Hex).unwrap(), "48 65");
    }
    #[test]
    fn utf8_tail_is_buffered() {
        let mut d = StreamDecoder::default();
        assert_eq!(d.decode(&[0xe4], &Encoding::Utf8).unwrap(), "");
        assert_eq!(d.decode(&[0xbd, 0xa0], &Encoding::Utf8).unwrap(), "你");
    }
}
