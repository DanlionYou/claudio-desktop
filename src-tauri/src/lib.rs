mod ai;
mod audio;
mod commands;
mod config;
mod speech;
mod state;

use std::sync::Mutex;
use std::time::Duration;

use tauri::Emitter;

use audio::get_audio_player;
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let theme = config::load_theme();
    let ai_config = config::load_ai_config();
    let speech_config = config::load_speech_config();

    // Load saved playlist
    let playlist_paths = config::load_playlist_paths();
    let playlist: Vec<state::TrackInfo> = playlist_paths
        .into_iter()
        .enumerate()
        .map(|(i, p)| state::extract_track_info(&p, i as u32))
        .collect();

    let app_state = Mutex::new(AppState {
        playlist,
        current_index: None,
        playback_state: state::PlaybackState::Stopped,
        volume: 0.8,
        theme,
        ai_config,
        speech_config,
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::play,
            commands::play_pause,
            commands::stop_audio,
            commands::next_track,
            commands::prev_track,
            commands::set_volume,
            commands::load_tracks,
            commands::add_tracks,
            commands::remove_track,
            commands::get_playlist,
            commands::set_theme,
            commands::get_theme,
            commands::get_playback_status,
            commands::chat_with_ai,
            commands::set_ai_api_key,
            commands::get_ai_config,
            commands::synthesize_speech,
            commands::recognize_speech,
            commands::set_speech_config,
            commands::get_speech_config,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            // Background task: emit playback progress every 500ms
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_millis(500)).await;

                    let progress = get_audio_player()
                        .lock()
                        .ok()
                        .and_then(|g| {
                            g.as_ref().map(|p| {
                                (p.get_elapsed(), p.current_duration(), p.is_empty())
                            })
                        });

                    if let Some((elapsed, total, _is_empty)) = progress {
                        let _ = handle.emit(
                            "playback-progress",
                            serde_json::json!({
                                "elapsed_secs": elapsed,
                                "total_secs": total,
                            }),
                        );
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
