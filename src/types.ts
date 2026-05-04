export interface TrackInfo {
  id: number;
  name: string;
  artist: string;
  path: string;
  duration_secs: number;
}

export type PlaybackState = "playing" | "paused" | "stopped";

export interface PlaybackStatus {
  state: PlaybackState;
  current_track_index: number | null;
  current_track: TrackInfo | null;
  elapsed_secs: number;
  total_secs: number;
  volume: number;
}

export interface ProgressPayload {
  elapsed_secs: number;
  total_secs: number;
}

export type Theme = "dark" | "light";

export interface AiMessage {
  role: string;
  content: string;
}

export interface AiChatRequest {
  messages: AiMessage[];
}

export interface MusicCard {
  track_name: string;
  artist: string;
  action: string;
  track_index: number | null;
}

export interface AiChatResponse {
  reply: string;
  music_card: MusicCard | null;
}

export interface AiConfig {
  api_key: string;
  model: string;
}

export interface SpeechConfig {
  access_key_id: string;
  access_key_secret: string;
  app_key: string;
  voice: string;
  enabled: boolean;
}

export interface SpeechResponse {
  audio_base64: string | null;
  text: string | null;
}
