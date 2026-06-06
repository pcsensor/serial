use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, State};

use super::encoding::Encoding;
use super::manager::{PortConfig, SerialManager};

pub struct SerialState(pub Mutex<SerialManager>);

#[derive(Serialize)]
pub struct PortInfo {
    pub port_name: String,
    pub port_type: String,
}

#[tauri::command]
pub fn list_ports() -> Vec<PortInfo> {
    SerialManager::list_ports()
        .into_iter()
        .map(|p| PortInfo {
            port_name: p.port_name,
            port_type: format!("{:?}", p.port_type),
        })
        .collect()
}

#[tauri::command]
pub fn open_port(
    state: State<SerialState>,
    app: AppHandle,
    config: PortConfig,
    encoding: Encoding,
) -> Result<(), String> {
    let mut manager = state.0.lock().map_err(|e| e.to_string())?;
    manager.open(&config)?;
    manager.start_receiving(app, encoding)?;
    Ok(())
}

#[tauri::command]
pub fn close_port(state: State<SerialState>) -> Result<(), String> {
    let mut manager = state.0.lock().map_err(|e| e.to_string())?;
    manager.close()
}

#[derive(Deserialize)]
pub struct SendRequest {
    pub content: String,
    pub encoding: Encoding,
}

#[tauri::command]
pub fn send_data(state: State<SerialState>, request: SendRequest) -> Result<usize, String> {
    let data = super::encoding::encode(&request.content, &request.encoding)?;
    let mut manager = state.0.lock().map_err(|e| e.to_string())?;
    manager.send(&data)
}

#[tauri::command]
pub fn is_port_open(state: State<SerialState>) -> bool {
    let manager = state.0.lock().unwrap();
    manager.is_open()
}
