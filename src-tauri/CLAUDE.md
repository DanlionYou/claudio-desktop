# CLAUDE.md - Claudio Desktop Music Player

## Build Commands
- Frontend type-check: `npx tsc --noEmit` (from claudio-desktop/)
- Rust check: `cd src-tauri && cargo check`
- Full build: `npm run tauri build` (from claudio-desktop/)
- Dev mode: `npm run tauri dev`

## Project Stack
- Tauri v2 + React + TypeScript + Rust
- Audio: rodio 0.20 (symphonia-all for MP3/FLAC/OGG/WAV/M4A/AAC)
- Window: 960x640, min 800x600
- Rust toolchain: stable-x86_64-pc-windows-gnu (LLVM MinGW)
- Linker: C:\tools\llvm-mingw\bin must be in PATH

## Architecture
- Rust backend: audio.rs (rodio wrapper), state.rs (AppState + models), commands.rs (Tauri IPC), config.rs (theme persistence)
- Frontend: React with hooks (usePlayer, useClock, useTheme), CSS variables for theming
- IPC: Tauri commands (invoke) + events (emit/listen)
- Audio: OnceLock<Mutex<Option<AudioPlayer>>> global singleton

## Key Files
- src-tauri/src/audio.rs - Audio playback engine
- src-tauri/src/commands.rs - All Tauri commands
- src-tauri/src/state.rs - Data models (TrackInfo, PlaybackStatus, AppState)
- src-tauri/src/lib.rs - App setup, progress emitter (500ms loop)
- src/types.ts - TypeScript interfaces
- src/commands.ts - Tauri invoke wrappers
- src/hooks/usePlayer.ts - Player state management
- src/styles/themes.css - Dark/light theme CSS variables

## Current Data Model
- TrackInfo: { id, name, artist, path, duration_secs }
- PlaybackStatus: { state, current_track_index, current_track, elapsed_secs, total_secs, volume }
- path field stores local file system path -> File::open -> rodio::Decoder
