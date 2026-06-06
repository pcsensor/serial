use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Encoding {
    Ascii,
    Hex,
    Utf8,
    Gbk,
}

pub fn encode(input: &str, encoding: &Encoding) -> Result<Vec<u8>, String> {
    match encoding {
        Encoding::Ascii => Ok(input.as_bytes().to_vec()),
        Encoding::Utf8 => Ok(input.as_bytes().to_vec()),
        Encoding::Hex => parse_hex(input),
        Encoding::Gbk => encode_gbk(input),
    }
}

pub fn decode(data: &[u8], encoding: &Encoding) -> Result<String, String> {
    match encoding {
        Encoding::Ascii => Ok(data.iter().map(|b| *b as char).collect()),
        Encoding::Utf8 => String::from_utf8(data.to_vec()).map_err(|e| e.to_string()),
        Encoding::Hex => Ok(format_hex(data)),
        Encoding::Gbk => decode_gbk(data),
    }
}

fn parse_hex(input: &str) -> Result<Vec<u8>, String> {
    let cleaned: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    if cleaned.len() % 2 != 0 {
        return Err("HEX 字符串长度必须为偶数".to_string());
    }
    (0..cleaned.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&cleaned[i..i + 2], 16)
                .map_err(|e| format!("无效的 HEX 字符: {}", e))
        })
        .collect()
}

fn format_hex(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

fn encode_gbk(input: &str) -> Result<Vec<u8>, String> {
    let (encoded, _, had_errors) = encoding_rs::GBK.encode(input);
    if had_errors {
        Err("GBK 编码失败：包含不支持的字符".to_string())
    } else {
        Ok(encoded.into_owned())
    }
}

fn decode_gbk(data: &[u8]) -> Result<String, String> {
    let (decoded, _, had_errors) = encoding_rs::GBK.decode(data);
    if had_errors {
        Err("GBK 解码失败：无效的字节序列".to_string())
    } else {
        Ok(decoded.into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_encode_decode() {
        let data = encode("Hello", &Encoding::Ascii).unwrap();
        assert_eq!(data, vec![72, 101, 108, 108, 111]);
        let text = decode(&data, &Encoding::Ascii).unwrap();
        assert_eq!(text, "Hello");
    }

    #[test]
    fn test_hex_encode_decode() {
        let data = encode("48 65 6C 6C 6F", &Encoding::Hex).unwrap();
        assert_eq!(data, vec![0x48, 0x65, 0x6C, 0x6C, 0x6F]);
        let text = decode(&data, &Encoding::Hex).unwrap();
        assert_eq!(text, "48 65 6C 6C 6F");
    }

    #[test]
    fn test_hex_without_spaces() {
        let data = encode("48656C6C6F", &Encoding::Hex).unwrap();
        assert_eq!(data, vec![0x48, 0x65, 0x6C, 0x6C, 0x6F]);
    }

    #[test]
    fn test_hex_odd_length_error() {
        let result = encode("486", &Encoding::Hex);
        assert!(result.is_err());
    }

    #[test]
    fn test_utf8_encode_decode() {
        let data = encode("你好世界", &Encoding::Utf8).unwrap();
        let text = decode(&data, &Encoding::Utf8).unwrap();
        assert_eq!(text, "你好世界");
    }

    #[test]
    fn test_gbk_encode_decode() {
        let data = encode("你好", &Encoding::Gbk).unwrap();
        let text = decode(&data, &Encoding::Gbk).unwrap();
        assert_eq!(text, "你好");
    }

    #[test]
    fn test_hex_format() {
        assert_eq!(format_hex(&[0x00, 0xFF, 0x0A]), "00 FF 0A");
    }
}
