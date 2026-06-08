mod export;
mod serial;

use serial::commands::SerialState;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .manage(SerialState(Mutex::new(
            serial::manager::SerialManager::new(),
        )))
        .invoke_handler(tauri::generate_handler![
            serial::commands::list_ports,
            serial::commands::open_port,
            serial::commands::close_port,
            serial::commands::send_data,
            serial::commands::is_port_open,
            export::exporter::export_data,
            export::exporter::save_dialog,
        ])
        .setup(|app| {
            let window = app.get_webview_window("main").expect("未找到 main 窗口");
            #[cfg(target_os = "macos")]
            {
                use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
                let _ = apply_vibrancy(&window, NSVisualEffectMaterial::Sidebar, None, Some(12.0));
            }
            #[cfg(target_os = "windows")]
            {
                use window_vibrancy::{apply_acrylic, apply_mica};
                if apply_mica(&window, None).is_err() {
                    let _ = apply_acrylic(&window, None);
                }
            }
            #[cfg(target_os = "linux")]
            {
                use window_vibrancy::apply_blur;
                let _ = apply_blur(&window, None);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用失败");
}
