use crate::state::{message_direction_label, Encoding, LogEntry, ReceivedMessage};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn entries_from_messages(messages: &[ReceivedMessage]) -> Vec<LogEntry> {
    messages
        .iter()
        .map(|message| LogEntry {
            timestamp: message.timestamp.clone(),
            direction: message_direction_label(&message.direction).into(),
            encoding: format_encoding(&message.encoding).into(),
            data: crate::state::visualize_serial_data(&message.data),
        })
        .collect()
}

fn format_encoding(encoding: &Encoding) -> &'static str {
    match encoding {
        Encoding::Ascii => "ASCII",
        Encoding::Hex => "HEX",
        Encoding::Utf8 => "UTF-8",
        Encoding::Gbk => "GBK",
    }
}

pub fn normalize_export_path(path: impl AsRef<Path>, format: &str) -> Result<PathBuf, String> {
    if !matches!(format, "txt" | "csv") {
        return Err("不支持的导出格式".into());
    }
    let mut path = path.as_ref().to_path_buf();
    match path.extension().and_then(|e| e.to_str()) {
        None | Some("") => {
            path.set_extension(format);
        }
        Some(ext) if !ext.eq_ignore_ascii_case(format) => {
            return Err(format!("路径扩展名不匹配: 期望 .{format}，实际 .{ext}"))
        }
        _ => {}
    }
    Ok(path)
}

pub fn export_txt(entries: &[LogEntry], path: impl AsRef<Path>) -> Result<(), String> {
    let content = entries
        .iter()
        .map(|e| format!("[{}] {}: {}", e.timestamp, e.direction, e.data))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(path, content).map_err(|e| format!("写入文件失败: {e}"))
}

pub fn export_csv(entries: &[LogEntry], path: impl AsRef<Path>) -> Result<(), String> {
    let mut writer = csv::WriterBuilder::new()
        .has_headers(true)
        .from_writer(Vec::new());
    writer
        .write_record(["时间", "方向", "编码", "数据"])
        .map_err(|e| e.to_string())?;
    for e in entries {
        writer
            .write_record([&e.timestamp, &e.direction, &e.encoding, &e.data])
            .map_err(|e| e.to_string())?;
    }
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend(writer.into_inner().map_err(|e| e.to_string())?);
    fs::write(path, bytes).map_err(|e| format!("写入文件失败: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn csv_quotes_every_field_safely() {
        let file = std::env::temp_dir().join(format!("serial-test-{}.csv", uuid::Uuid::new_v4()));
        export_csv(
            &[LogEntry {
                timestamp: "t,1".into(),
                direction: "收到".into(),
                encoding: "UTF-8".into(),
                data: "a\"b\nc".into(),
            }],
            &file,
        )
        .unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("\"t,1\""));
        assert!(text.contains("\"a\"\"b\nc\""));
        let _ = std::fs::remove_file(file);
    }
    #[test]
    fn extension_is_normalized_and_validated() {
        assert_eq!(
            normalize_export_path("log", "csv")
                .unwrap()
                .extension()
                .unwrap(),
            "csv"
        );
        assert!(normalize_export_path("log.txt", "csv").is_err());
    }
}
