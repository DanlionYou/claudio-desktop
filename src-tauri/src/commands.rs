use std::sync::Mutex;

use tauri::{AppHandle, Emitter, State};

use crate::audio::{self, AudioPlayer};
use crate::config;
use crate::state::{self as st, AppState, PlaybackState, PlaybackStatus, Theme, TrackInfo};

/// Build status from AppState only — does NOT lock AudioPlayer (avoids deadlock).
/// Elapsed/total are best-effort from state; real-time progress comes via events.
pub fn get_status(state: &AppState) -> PlaybackStatus {
    PlaybackStatus {
        state: state.playback_state.clone(),
        current_track_index: state.current_index,
        current_track: state
            .current_index
            .and_then(|i| state.playlist.get(i).cloned()),
        elapsed_secs: 0.0,
        volume: state.volume,
        total_secs: state
            .current_index
            .and_then(|i| state.playlist.get(i))
            .map(|t| t.duration_secs)
            .unwrap_or(0.0),
    }
}

#[tauri::command]
pub fn load_tracks(paths: Vec<String>) -> Result<Vec<TrackInfo>, String> {
    let tracks: Vec<TrackInfo> = paths
        .into_iter()
        .enumerate()
        .map(|(i, p)| st::extract_track_info(&p, i as u32))
        .collect();
    Ok(tracks)
}

#[tauri::command]
pub fn add_tracks(
    paths: Vec<String>,
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<TrackInfo>, String> {
    let mut app_state = state.lock().map_err(|e| e.to_string())?;
    let start_id = app_state.playlist.len() as u32;
    let tracks: Vec<TrackInfo> = paths
        .into_iter()
        .enumerate()
        .map(|(i, p)| st::extract_track_info(&p, start_id + i as u32))
        .collect();
    app_state.playlist.extend(tracks.clone());
    config::save_playlist(&app_state.playlist)?;
    Ok(tracks)
}

#[tauri::command]
pub fn play(
    index: usize,
    state: State<'_, Mutex<AppState>>,
    app: AppHandle,
) -> Result<PlaybackStatus, String> {
    let mut app_state = state.lock().map_err(|e| e.to_string())?;

    if index >= app_state.playlist.len() {
        return Err("索引超出播放列表范围".to_string());
    }

    let track = app_state.playlist[index].clone();

    let mut player_lock = audio::get_audio_player().lock().map_err(|e| e.to_string())?;
    if player_lock.is_none() {
        *player_lock = Some(AudioPlayer::new()?);
    }
    let player = player_lock.as_mut().unwrap();
    let duration = player.play(&track.path)?;

    app_state.current_index = Some(index);
    app_state.playback_state = PlaybackState::Playing;
    player.set_volume(app_state.volume);

    let status = PlaybackStatus {
        state: PlaybackState::Playing,
        current_track_index: Some(index),
        current_track: Some(TrackInfo {
            duration_secs: duration,
            ..track
        }),
        elapsed_secs: 0.0,
        volume: app_state.volume,
        total_secs: duration,
    };

    // Drop locks before emitting
    drop(player_lock);
    drop(app_state);

    let _ = app.emit("playback-state-changed", &status);
    Ok(status)
}

#[tauri::command]
pub fn play_pause(
    state: State<'_, Mutex<AppState>>,
    app: AppHandle,
) -> Result<PlaybackStatus, String> {
    let mut app_state = state.lock().map_err(|e| e.to_string())?;
    let mut player_lock = audio::get_audio_player().lock().map_err(|e| e.to_string())?;

    if let Some(player) = player_lock.as_mut() {
        match app_state.playback_state {
            PlaybackState::Playing => {
                player.pause();
                app_state.playback_state = PlaybackState::Paused;
            }
            PlaybackState::Paused => {
                player.resume();
                app_state.playback_state = PlaybackState::Playing;
            }
            PlaybackState::Stopped => {
                if let Some(idx) = app_state.current_index {
                    if idx < app_state.playlist.len() {
                        let track = app_state.playlist[idx].clone();
                        let _ = player.play(&track.path);
                        player.set_volume(app_state.volume);
                        app_state.playback_state = PlaybackState::Playing;
                    }
                }
            }
        }
    }

    // Drop player_lock BEFORE calling get_status (avoid deadlock)
    drop(player_lock);

    let status = get_status(&app_state);
    let _ = app.emit("playback-state-changed", &status);
    Ok(status)
}

#[tauri::command]
pub fn stop_audio(
    state: State<'_, Mutex<AppState>>,
    app: AppHandle,
) -> Result<PlaybackStatus, String> {
    let mut app_state = state.lock().map_err(|e| e.to_string())?;

    // Lock audio player, stop, then drop lock
    if let Ok(mut player_lock) = audio::get_audio_player().lock() {
        if let Some(player) = player_lock.as_mut() {
            player.stop();
        }
        drop(player_lock);
    }

    app_state.playback_state = PlaybackState::Stopped;
    let status = get_status(&app_state);
    let _ = app.emit("playback-state-changed", &status);
    Ok(status)
}

#[tauri::command]
pub fn next_track(
    state: State<'_, Mutex<AppState>>,
    app: AppHandle,
) -> Result<Option<PlaybackStatus>, String> {
    let mut app_state = state.lock().map_err(|e| e.to_string())?;
    if app_state.playlist.is_empty() {
        return Ok(None);
    }

    let next_idx = match app_state.current_index {
        Some(i) if i + 1 < app_state.playlist.len() => i + 1,
        Some(_) => 0,
        None => 0,
    };

    let track = app_state.playlist[next_idx].clone();
    app_state.current_index = Some(next_idx);
    app_state.playback_state = PlaybackState::Playing;

    if let Ok(mut player_lock) = audio::get_audio_player().lock() {
        if let Some(player) = player_lock.as_mut() {
            let _ = player.play(&track.path);
            player.set_volume(app_state.volume);
        }
        drop(player_lock);
    }

    let status = get_status(&app_state);
    let _ = app.emit("playback-state-changed", &status);
    Ok(Some(status))
}

#[tauri::command]
pub fn prev_track(
    state: State<'_, Mutex<AppState>>,
    app: AppHandle,
) -> Result<Option<PlaybackStatus>, String> {
    let mut app_state = state.lock().map_err(|e| e.to_string())?;
    if app_state.playlist.is_empty() {
        return Ok(None);
    }

    let prev_idx = match app_state.current_index {
        Some(i) if i > 0 => i - 1,
        Some(_) => app_state.playlist.len() - 1,
        None => 0,
    };

    let track = app_state.playlist[prev_idx].clone();
    app_state.current_index = Some(prev_idx);
    app_state.playback_state = PlaybackState::Playing;

    if let Ok(mut player_lock) = audio::get_audio_player().lock() {
        if let Some(player) = player_lock.as_mut() {
            let _ = player.play(&track.path);
            player.set_volume(app_state.volume);
        }
        drop(player_lock);
    }

    let status = get_status(&app_state);
    let _ = app.emit("playback-state-changed", &status);
    Ok(Some(status))
}

#[tauri::command]
pub fn set_volume(
    volume: f64,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let mut app_state = state.lock().map_err(|e| e.to_string())?;
    app_state.volume = volume.clamp(0.0, 1.0);
    if let Ok(player_lock) = audio::get_audio_player().lock() {
        if let Some(player) = player_lock.as_ref() {
            player.set_volume(app_state.volume);
        }
        // Lock dropped here
    }
    Ok(())
}

#[tauri::command]
pub fn remove_track(
    index: usize,
    state: State<'_, Mutex<AppState>>,
    app: AppHandle,
) -> Result<(), String> {
    let mut app_state = state.lock().map_err(|e| e.to_string())?;

    let was_current = app_state.current_index == Some(index);
    let shifted_before = app_state
        .current_index
        .map(|ci| index < ci)
        .unwrap_or(false);

    app_state.playlist.remove(index);

    config::save_playlist(&app_state.playlist)?;

    if was_current {
        if let Ok(mut player_lock) = audio::get_audio_player().lock() {
            if let Some(player) = player_lock.as_mut() {
                player.stop();
            }
        }
        app_state.playback_state = PlaybackState::Stopped;
        app_state.current_index = None;
    } else if shifted_before {
        app_state.current_index = app_state.current_index.map(|ci| ci - 1);
    }

    let _ = app.emit("playlist-updated", &app_state.playlist);
    Ok(())
}

#[tauri::command]
pub fn get_playlist(state: State<'_, Mutex<AppState>>) -> Result<Vec<TrackInfo>, String> {
    let app_state = state.lock().map_err(|e| e.to_string())?;
    Ok(app_state.playlist.clone())
}

#[tauri::command]
pub fn set_theme(
    theme_str: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let theme = Theme::from_str(&theme_str);
    let mut app_state = state.lock().map_err(|e| e.to_string())?;
    app_state.theme = theme.clone();
    config::save_theme(&theme)?;
    Ok(())
}

#[tauri::command]
pub fn get_theme() -> Result<String, String> {
    let theme = config::load_theme();
    Ok(theme.as_str().to_string())
}

#[tauri::command]
pub fn get_playback_status(
    state: State<'_, Mutex<AppState>>,
) -> Result<PlaybackStatus, String> {
    let app_state = state.lock().map_err(|e| e.to_string())?;
    Ok(get_status(&app_state))
}

#[tauri::command]
pub async fn chat_with_ai(
    request: crate::state::AiChatRequest,
    state: State<'_, Mutex<AppState>>,
    app: AppHandle,
) -> Result<crate::state::AiChatResponse, String> {
    let api_key;
    let model;
    {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        api_key = app_state.ai_config.api_key.clone();
        model = app_state.ai_config.model.clone();
    }
    crate::ai::chat_with_ai(request, api_key, model, &state, &app).await
}

#[tauri::command]
pub fn set_ai_api_key(
    api_key: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let mut app_state = state.lock().map_err(|e| e.to_string())?;
    app_state.ai_config.api_key = api_key.clone();
    app_state.ai_config.model = "deepseek-chat".to_string();
    crate::config::save_ai_config(&app_state.ai_config)?;
    Ok(())
}

#[tauri::command]
pub fn get_ai_config(
    state: State<'_, Mutex<AppState>>,
) -> Result<crate::state::AiConfig, String> {
    let app_state = state.lock().map_err(|e| e.to_string())?;
    Ok(app_state.ai_config.clone())
}

// ── Speech commands ──

#[tauri::command]
pub async fn synthesize_speech(
    text: String,
    voice: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<crate::state::SpeechResponse, String> {
    let (ak_id, ak_secret, app_key);
    {
        let cfg = &state.lock().map_err(|e| e.to_string())?.speech_config;
        ak_id = cfg.access_key_id.clone();
        ak_secret = cfg.access_key_secret.clone();
        app_key = cfg.app_key.clone();
    }
    let audio_base64 =
        crate::speech::synthesize_speech(&text, &ak_id, &ak_secret, &app_key, &voice).await?;
    Ok(crate::state::SpeechResponse {
        audio_base64: Some(audio_base64),
        text: None,
    })
}

#[tauri::command]
pub async fn recognize_speech(
    audio: Vec<u8>,
    state: State<'_, Mutex<AppState>>,
) -> Result<crate::state::SpeechResponse, String> {
    let (ak_id, ak_secret, app_key);
    {
        let cfg = &state.lock().map_err(|e| e.to_string())?.speech_config;
        ak_id = cfg.access_key_id.clone();
        ak_secret = cfg.access_key_secret.clone();
        app_key = cfg.app_key.clone();
    }
    let text = crate::speech::recognize_speech(audio, &ak_id, &ak_secret, &app_key).await?;
    Ok(crate::state::SpeechResponse {
        audio_base64: None,
        text: Some(text),
    })
}

#[tauri::command]
pub fn set_speech_config(
    mut speech: crate::state::SpeechConfig,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    // Sanitize voice before saving
    if speech.voice.starts_with("long") || speech.voice.starts_with("cosyvoice") {
        speech.voice = "zhixiaoxia".to_string();
    }
    let mut app_state = state.lock().map_err(|e| e.to_string())?;
    app_state.speech_config = speech.clone();
    crate::config::save_speech_config(&speech)?;
    Ok(())
}

#[tauri::command]
pub fn get_speech_config(
    state: State<'_, Mutex<AppState>>,
) -> Result<crate::state::SpeechConfig, String> {
    let mut app_state = state.lock().map_err(|e| e.to_string())?;
    // Replace CosyVoice-only voices (from 百炼) with a supported ISI voice
    if app_state.speech_config.voice.starts_with("long")
        || app_state.speech_config.voice.starts_with("cosyvoice")
    {
        app_state.speech_config.voice = "zhixiaoxia".to_string();
    }
    Ok(app_state.speech_config.clone())
}
