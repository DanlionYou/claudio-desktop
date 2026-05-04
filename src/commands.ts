import { invoke } from "@tauri-apps/api/core";
import type { TrackInfo, PlaybackStatus, AiChatRequest, AiChatResponse, AiConfig, SpeechConfig, SpeechResponse } from "./types";

export const commands = {
  openFileDialog: (): Promise<string[]> => {
    // Use the dialog plugin directly
    return invoke<string[]>("open_file_dialog");
  },

  addTracks: (paths: string[]): Promise<TrackInfo[]> => {
    return invoke<TrackInfo[]>("add_tracks", { paths });
  },

  loadTracks: (paths: string[]): Promise<TrackInfo[]> => {
    return invoke<TrackInfo[]>("load_tracks", { paths });
  },

  play: (index: number): Promise<PlaybackStatus> => {
    return invoke<PlaybackStatus>("play", { index });
  },

  playPause: (): Promise<PlaybackStatus> => {
    return invoke<PlaybackStatus>("play_pause");
  },

  stopAudio: (): Promise<PlaybackStatus> => {
    return invoke<PlaybackStatus>("stop_audio");
  },

  nextTrack: (): Promise<PlaybackStatus | null> => {
    return invoke<PlaybackStatus | null>("next_track");
  },

  prevTrack: (): Promise<PlaybackStatus | null> => {
    return invoke<PlaybackStatus | null>("prev_track");
  },

  setVolume: (volume: number): Promise<void> => {
    return invoke<void>("set_volume", { volume });
  },

  removeTrack: (index: number): Promise<void> => {
    return invoke<void>("remove_track", { index });
  },

  getPlaylist: (): Promise<TrackInfo[]> => {
    return invoke<TrackInfo[]>("get_playlist");
  },

  setTheme: (theme: string): Promise<void> => {
    return invoke<void>("set_theme", { themeStr: theme });
  },

  getTheme: (): Promise<string> => {
    return invoke<string>("get_theme");
  },

  getPlaybackStatus: (): Promise<PlaybackStatus> => {
    return invoke<PlaybackStatus>("get_playback_status");
  },

  chatWithAI: (request: AiChatRequest): Promise<AiChatResponse> => {
    return invoke<AiChatResponse>("chat_with_ai", { request });
  },

  setAiApiKey: (apiKey: string): Promise<void> => {
    return invoke<void>("set_ai_api_key", { apiKey });
  },

  getAiConfig: (): Promise<AiConfig> => {
    return invoke<AiConfig>("get_ai_config");
  },

  // ── Speech ──
  synthesizeSpeech: (text: string, voice: string): Promise<SpeechResponse> => {
    return invoke<SpeechResponse>("synthesize_speech", { text, voice });
  },

  recognizeSpeech: (audio: number[]): Promise<SpeechResponse> => {
    return invoke<SpeechResponse>("recognize_speech", { audio });
  },

  setSpeechConfig: (speech: SpeechConfig): Promise<void> => {
    return invoke<void>("set_speech_config", { speech });
  },

  getSpeechConfig: (): Promise<SpeechConfig> => {
    return invoke<SpeechConfig>("get_speech_config");
  },
};
