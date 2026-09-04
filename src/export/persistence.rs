use crate::state::{PortConfig, PresetCommand};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

static SAVE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SavedSettings {
    pub port_config: PortConfig,
    pub presets: Vec<PresetCommand>,
}

pub fn settings_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("serial-debugger").join("settings.json"))
}

pub fn load() -> SavedSettings {
    let Some(path) = settings_path() else {
        return SavedSettings::default();
    };
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save(settings: &SavedSettings) -> Result<(), String> {
    let _guard = SAVE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "配置锁已损坏".to_string())?;
    let path = settings_path().ok_or("无法定位配置目录")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {e}"))?;
    }
    let text =
        serde_json::to_string_pretty(settings).map_err(|e| format!("序列化配置失败: {e}"))?;
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, text).map_err(|e| format!("写入配置失败: {e}"))?;
    fs::rename(&temp, &path).map_err(|e| format!("保存配置失败: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_settings_are_usable() {
        let settings = SavedSettings::default();
        assert_eq!(settings.port_config.baud_rate, 115200);
    }
}
