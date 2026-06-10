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

/// 返回某导出格式在保存对话框中应使用的筛选器（显示名称, 扩展名）。
///
/// 这是格式的“单一事实来源”：对话框筛选器必须与用户在界面下拉框中选择的
/// 格式一致，否则会出现“选了 CSV 却存成 .txt”的问题——因为原生保存对话框
/// 会按其默认筛选器追加扩展名，而非按界面下拉框。
fn dialog_filter_for(format: &str) -> Result<(&'static str, &'static str), String> {
    match format {
        "txt" => Ok(("文本文件", "txt")),
        "csv" => Ok(("CSV 文件", "csv")),
        _ => Err("不支持的导出格式".to_string()),
    }
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
    use super::{dialog_filter_for, normalize_export_path};

    #[test]
    fn dialog_filter_matches_selected_format() {
        // 选择 CSV 时，对话框必须使用 csv 筛选器（而非默认的 txt），
        // 否则会出现“选 CSV 却导出 .txt”的回归。
        assert_eq!(dialog_filter_for("csv").unwrap(), ("CSV 文件", "csv"));
        assert_eq!(dialog_filter_for("txt").unwrap(), ("文本文件", "txt"));
    }

    #[test]
    fn dialog_filter_rejects_unknown_format() {
        assert!(dialog_filter_for("pdf").is_err());
    }

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
pub async fn save_dialog(app: tauri::AppHandle, format: String) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    // 仅添加与所选格式匹配的筛选器，并预填对应扩展名的文件名，
    // 确保对话框返回的路径扩展名与界面下拉框一致。
    let (name, ext) = dialog_filter_for(&format)?;
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter(name, &[ext])
        .set_file_name(format!("export.{}", ext))
        .save_file(move |path| {
            let _ = tx.send(path.map(|p| p.to_string()));
        });
    rx.recv().map_err(|e| e.to_string())
}
