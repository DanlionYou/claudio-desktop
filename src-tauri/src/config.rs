use std::path::PathBuf;

use serde_json;

use crate::state::{AiConfig, SpeechConfig, Theme, TrackInfo};

fn config_dir() -> PathBuf {
    let mut path = dirs_next::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("claudio-desktop");
    std::fs::create_dir_all(&path).ok();
    path
}

pub fn config_path() -> PathBuf {
    let mut path = config_dir();
    path.push("config.json");
    path
}

fn read_config() -> serde_json::Value {
    let path = config_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or(serde_json::json!({}))
}

fn write_config(config: &serde_json::Value) -> Result<(), String> {
    let path = config_path();
    std::fs::write(&path, serde_json::to_string_pretty(config).unwrap())
        .map_err(|e| format!("保存配置失败: {}", e))
}

pub fn save_theme(theme: &Theme) -> Result<(), String> {
    let mut config = read_config();
    config["theme"] = serde_json::json!(theme.as_str());
    write_config(&config)
}

pub fn load_theme() -> Theme {
    let config = read_config();
    match config.get("theme").and_then(|v| v.as_str()) {
        Some(t) => Theme::from_str(t),
        None => Theme::Dark,
    }
}

pub fn save_ai_config(ai_config: &AiConfig) -> Result<(), String> {
    let mut config = read_config();
    config["ai"] = serde_json::json!({
        "api_key": ai_config.api_key,
        "model": ai_config.model,
    });
    write_config(&config)
}

pub fn load_ai_config() -> AiConfig {
    let config = read_config();
    match config.get("ai") {
        Some(ai) => AiConfig {
            api_key: ai.get("api_key").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            model: ai.get("model").and_then(|v| v.as_str()).unwrap_or("deepseek-chat").to_string(),
        },
        None => AiConfig::default(),
    }
}

pub fn save_speech_config(speech: &SpeechConfig) -> Result<(), String> {
    let mut config = read_config();
    config["speech"] = serde_json::json!({
        "access_key_id": speech.access_key_id,
        "access_key_secret": speech.access_key_secret,
        "app_key": speech.app_key,
        "voice": speech.voice,
        "enabled": speech.enabled,
    });
    write_config(&config)
}

pub fn load_speech_config() -> SpeechConfig {
    let config = read_config();
    match config.get("speech") {
        Some(s) => SpeechConfig {
            access_key_id: s.get("access_key_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            access_key_secret: s.get("access_key_secret").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            app_key: s.get("app_key").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            voice: s.get("voice").and_then(|v| v.as_str()).unwrap_or("zhixiaoxia").to_string(),
            enabled: s.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false),
        },
        None => SpeechConfig::default(),
    }
}

// ── Playlist persistence ──

/// Save playlist track paths to config.json
pub fn save_playlist(tracks: &[TrackInfo]) -> Result<(), String> {
    let mut config = read_config();
    let paths: Vec<&str> = tracks.iter().map(|t| t.path.as_str()).collect();
    config["playlist"] = serde_json::json!(paths);
    write_config(&config)
}

/// Load saved playlist file paths from config.json
pub fn load_playlist_paths() -> Vec<String> {
    let config = read_config();
    config
        .get("playlist")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}
