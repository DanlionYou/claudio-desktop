use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackInfo {
    pub id: u32,
    pub name: String,
    pub artist: String,
    pub path: String,
    pub duration_secs: f64,
}

/// Derive track info from a file path by parsing the filename.
/// Expects "Artist - Name.ext" format; falls back to "Unknown Artist" / filename.
pub fn extract_track_info(path_str: &str, id: u32) -> TrackInfo {
    let path = Path::new(path_str);
    let filename = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let (artist, name) = if let Some(dash_pos) = filename.find(" - ") {
        let a = filename[..dash_pos].trim().to_string();
        let n = filename[dash_pos + 3..].trim().to_string();
        (a, n)
    } else {
        ("Unknown Artist".to_string(), filename)
    };

    TrackInfo {
        id,
        name,
        artist,
        path: path_str.to_string(),
        duration_secs: 0.0,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackStatus {
    pub state: PlaybackState,
    pub current_track_index: Option<usize>,
    pub current_track: Option<TrackInfo>,
    pub elapsed_secs: f64,
    pub total_secs: f64,
    pub volume: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    pub fn as_str(&self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "light" => Theme::Light,
            _ => Theme::Dark,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub api_key: String,
    pub model: String,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: "deepseek-chat".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiMessage {
    pub role: String,       // "user" | "assistant" | "system"
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiChatRequest {
    pub messages: Vec<AiMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicCard {
    pub track_name: String,
    pub artist: String,
    pub action: String,     // "play" | "recommend" | "list"
    pub track_index: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiChatResponse {
    pub reply: String,
    pub music_card: Option<MusicCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechConfig {
    pub access_key_id: String,
    pub access_key_secret: String,
    pub app_key: String,
    pub voice: String,         // CosyVoice 音色
    pub enabled: bool,
}

impl Default for SpeechConfig {
    fn default() -> Self {
        Self {
            access_key_id: String::new(),
            access_key_secret: String::new(),
            app_key: String::new(),
            voice: "zhixiaoxia".to_string(),
            enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechResponse {
    pub audio_base64: Option<String>,  // TTS 结果
    pub text: Option<String>,          // ASR 结果
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub playlist: Vec<TrackInfo>,
    pub current_index: Option<usize>,
    pub playback_state: PlaybackState,
    pub volume: f64,
    pub theme: Theme,
    pub ai_config: AiConfig,
    pub speech_config: SpeechConfig,
}
