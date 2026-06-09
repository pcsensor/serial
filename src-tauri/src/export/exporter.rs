use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

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

/// 校验导出路径的扩展名与 format 参数是否一致，防止前端绕过对话框写入任意路径
fn validate_export_path(path: &str, format: &str) -> Result<(), String> {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if ext.to_lowercase() != format {
        return Err(format!(
            "路径扩展名不匹配: 期望 .{}，实际 .{}",
            format, ext
        ));
    }
    Ok(())
}

#[tauri::command]
pub fn export_data(entries: Vec<LogEntry>, path: String, format: String) -> Result<(), String> {
    match format.as_str() {
        "txt" => {
            validate_export_path(&path, "txt")?;
            export_txt(&entries, &path)
        }
        "csv" => {
            validate_export_path(&path, "csv")?;
            export_csv(&entries, &path)
        }
        _ => Err("不支持的导出格式".to_string()),
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

