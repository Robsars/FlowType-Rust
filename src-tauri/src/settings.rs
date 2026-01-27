use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri::Manager;
use log::{info, error};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub auto_space: bool,
    pub silence_timeout: u64,
    pub allow_commands: bool,
    pub disable_punctuation: bool,
    pub shortcuts: HashMap<String, String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        let mut shortcuts = HashMap::new();
        shortcuts.insert("delete".to_string(), "[BACKSPACE]".to_string());
        shortcuts.insert("backspace".to_string(), "[BACKSPACE]".to_string());
        shortcuts.insert("delete that".to_string(), "[DELETE_LINE]".to_string());
        shortcuts.insert("new line".to_string(), "[ENTER]".to_string());
        shortcuts.insert("enter".to_string(), "[ENTER]".to_string());
        shortcuts.insert("space".to_string(), " ".to_string());
        
        Self {
            auto_space: true,
            silence_timeout: 500,
            allow_commands: true,
            disable_punctuation: false,
            shortcuts,
        }
    }
}

pub struct SettingsManager {
    file_path: PathBuf,
}

impl SettingsManager {
    pub fn new(app: &AppHandle) -> Self {
        let config_dir = app.path().app_config_dir().unwrap_or_else(|_| PathBuf::from("."));
        if !config_dir.exists() {
            let _ = fs::create_dir_all(&config_dir);
        }
        let file_path = config_dir.join("settings.json");
        Self { file_path }
    }

    pub fn load(&self) -> AppSettings {
        if self.file_path.exists() {
            match fs::read_to_string(&self.file_path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(settings) => {
                        info!("Settings loaded from {:?}", self.file_path);
                        return settings;
                    },
                    Err(e) => error!("Failed to parse settings: {}", e),
                },
                Err(e) => error!("Failed to read settings file: {}", e),
            }
        }
        info!("Using default settings and saving to {:?}", self.file_path);
        let defaults = AppSettings::default();
        self.save(&defaults);
        defaults
    }

    pub fn save(&self, settings: &AppSettings) {
        match serde_json::to_string_pretty(settings) {
            Ok(json) => {
                if let Err(e) = fs::write(&self.file_path, json) {
                    error!("Failed to write settings: {}", e);
                } else {
                    info!("Settings saved.");
                }
            },
            Err(e) => error!("Failed to serialize settings: {}", e),
        }
    }
}
