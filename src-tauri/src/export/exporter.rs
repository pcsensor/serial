use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub direction: String,
    pub encoding: String,
    pub data: String,
}

pub fn export_txt(entries: &[LogEntry], path: &str) -> Result<(), String> {
    let content: String = entries
        .iter()
        .map(|e| format!("[{}] {}: {}", e.timestamp, e.direction, e.data))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(Path::new(path), content).map_err(|e| format!("写入文件失败: {}", e))
}

pub fn export_csv(entries: &[LogEntry], path: &str) -> Result<(), String> {
    // UTF-8 BOM：确保 Windows Excel 以 UTF-8 打开，防止中文乱码
    let mut content = String::from("\u{FEFF}时间,方向,编码,数据\n");
    for e in entries {
        let escaped_data = e.data.replace('"', "\"\"");
        content.push_str(&format!(
            "{},{},{},\"{}\"\n",
            e.timestamp, e.direction, e.encoding, escaped_data
        ));
    }
    fs::write(Path::new(path), content).map_err(|e| format!("写入文件失败: {}", e))
}

/// 规范化导出路径：Windows 保存对话框切换筛选器时可能只返回无扩展名路径。
fn normalize_export_path(path: &str, format: &str) -> Result<PathBuf, String> {
    let mut path = PathBuf::from(path);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    if ext.is_empty() {
        path.set_extension(format);
        return Ok(path);
    }

    if !ext.eq_ignore_ascii_case(format) {
        return Err(format!("路径扩展名不匹配: 期望 .{}，实际 .{}", format, ext));
    }

    Ok(path)
}

#[tauri::command]
pub fn export_data(entries: Vec<LogEntry>, path: String, format: String) -> Result<(), String> {
    match format.as_str() {
        "txt" => {
            let path = normalize_export_path(&path, "txt")?;
            export_txt(&entries, &path.to_string_lossy())
        }
        "csv" => {
            let path = normalize_export_path(&path, "csv")?;
            export_csv(&entries, &path.to_string_lossy())
        }
        _ => Err("不支持的导出格式".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_export_path;

    #[test]
    fn normalize_export_path_appends_missing_csv_extension() {
        let path = normalize_export_path("log", "csv").expect("path should normalize");

        assert_eq!(path.to_string_lossy(), "log.csv");
    }

    #[test]
    fn normalize_export_path_rejects_mismatched_extension() {
        let err = normalize_export_path("log.txt", "csv").expect_err("extension must match format");

        assert!(err.contains("路径扩展名不匹配"));
    }
}

#[tauri::command]
pub async fn save_dialog(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("文本文件", &["txt"])
        .add_filter("CSV 文件", &["csv"])
        .save_file(move |path| {
            let _ = tx.send(path.map(|p| p.to_string()));
        });
    rx.recv().map_err(|e| e.to_string())
}
