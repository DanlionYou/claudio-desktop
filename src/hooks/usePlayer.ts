import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { commands } from "../commands";
import type { PlaybackStatus, TrackInfo } from "../types";

const DEFAULT_STATUS: PlaybackStatus = {
  state: "stopped",
  current_track_index: null,
  current_track: null,
  elapsed_secs: 0,
  total_secs: 0,
  volume: 0.8,
};

export function usePlayer() {
  const [status, setStatus] = useState<PlaybackStatus>(DEFAULT_STATUS);
  const [playlist, setPlaylist] = useState<TrackInfo[]>([]);
  const [progress, setProgress] = useState({ elapsed_secs: 0, total_secs: 0 });
  const statusRef = useRef(status);
  const progressRef = useRef(progress);
  const playlistRef = useRef(playlist);

  // Keep refs in sync
  useEffect(() => { statusRef.current = status; }, [status]);
  useEffect(() => { progressRef.current = progress; }, [progress]);
  useEffect(() => { playlistRef.current = playlist; }, [playlist]);

  useEffect(() => {
    // Listen for playback state changes
    const unlisten1 = listen<PlaybackStatus>("playback-state-changed", (e) => {
      setStatus(e.payload);
    });

    // Listen for progress updates
    const unlisten2 = listen<{ elapsed_secs: number; total_secs: number }>(
      "playback-progress",
      (e) => {
        setProgress(e.payload);

        // Auto-advance: if track finished, go to next
        const { elapsed_secs, total_secs } = e.payload;
        if (
          total_secs > 1 &&
          elapsed_secs >= total_secs - 0.3 &&
          statusRef.current.state === "playing"
        ) {
          const s = statusRef.current;
          const pl = playlistRef.current;
          if (s.current_track_index !== null) {
            const nextIdx = s.current_track_index + 1;
            if (nextIdx < pl.length) {
              commands.play(nextIdx).then(setStatus).catch(() => {});
            } else {
              commands.stopAudio().then(setStatus).catch(() => {});
            }
          }
        }
      }
    );

    // Listen for playlist changes
    const unlisten3 = listen<TrackInfo[]>("playlist-updated", (e) => {
      setPlaylist(e.payload);
    });

    // Load initial state
    commands.getPlaybackStatus().then(setStatus).catch(() => {});
    commands.getPlaylist().then(setPlaylist).catch(() => {});

    return () => {
      unlisten1.then((f) => f());
      unlisten2.then((f) => f());
      unlisten3.then((f) => f());
    };
  }, []);

  const playTrack = useCallback(async (index: number) => {
    try {
      const s = await commands.play(index);
      setStatus(s);
    } catch (e) {
      console.error("Play error:", e);
    }
  }, []);

  const togglePlayPause = useCallback(async () => {
    try {
      const s = await commands.playPause();
      setStatus(s);
    } catch (e) {
      console.error("Play/pause error:", e);
    }
  }, []);

  const stop = useCallback(async () => {
    try {
      const s = await commands.stopAudio();
      setStatus(s);
    } catch (e) {
      console.error("Stop error:", e);
    }
  }, []);

  const next = useCallback(async () => {
    try {
      const s = await commands.nextTrack();
      if (s) setStatus(s);
    } catch (e) {
      console.error("Next error:", e);
    }
  }, []);

  const prev = useCallback(async () => {
    try {
      const s = await commands.prevTrack();
      if (s) setStatus(s);
    } catch (e) {
      console.error("Prev error:", e);
    }
  }, []);

  const setVolume = useCallback(async (vol: number) => {
    try {
      await commands.setVolume(vol);
      setStatus((s) => ({ ...s, volume: vol }));
    } catch (e) {
      console.error("Volume error:", e);
    }
  }, []);

  const addFiles = useCallback(async (paths: string[]) => {
    try {
      const newTracks = await commands.addTracks(paths);
      setPlaylist((p) => [...p, ...newTracks]);
    } catch (e) {
      console.error("Add tracks error:", e);
    }
  }, []);

  const removeTrack = useCallback(async (index: number) => {
    try {
      await commands.removeTrack(index);
      setPlaylist((p) => p.filter((_, i) => i !== index));
    } catch (e) {
      console.error("Remove track error:", e);
    }
  }, []);

  return {
    status,
    playlist,
    progress,
    playTrack,
    togglePlayPause,
    stop,
    next,
    prev,
    setVolume,
    addFiles,
    removeTrack,
  };
}
