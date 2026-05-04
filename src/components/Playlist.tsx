import { useRef } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import type { TrackInfo } from "../types";
import "./Playlist.css";

interface PlaylistProps {
  tracks: TrackInfo[];
  currentIndex: number | null;
  onPlay: (index: number) => void;
  onRemove: (index: number) => void;
  onFilesAdded: (paths: string[]) => void;
}

export function Playlist({
  tracks,
  currentIndex,
  onPlay,
  onRemove,
  onFilesAdded,
}: PlaylistProps) {
  const listRef = useRef<HTMLDivElement>(null);

  const handleAddFiles = async () => {
    try {
      const selected = await open({
        multiple: true,
        filters: [
          {
            name: "Audio",
            extensions: ["mp3", "flac", "ogg", "wav", "m4a", "aac"],
          },
        ],
      });
      if (selected) {
        const paths = Array.isArray(selected) ? selected : [selected];
        onFilesAdded(paths);
      }
    } catch (e) {
      console.error("Dialog error:", e);
    }
  };

  return (
    <div className="playlist">
      <div className="playlist-header">
        <span className="playlist-title">播放列表</span>
        <button className="add-btn" onClick={handleAddFiles} title="添加音乐">
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" className="add-icon">
            <line x1="8" y1="3" x2="8" y2="13"/>
            <line x1="3" y1="8" x2="13" y2="8"/>
          </svg>
        </button>
      </div>
      {tracks.length === 0 ? (
        <div className="playlist-empty" onClick={handleAddFiles} title="点击添加音乐">
          <div className="empty-icon">♪</div>
          <p>点击此处导入音乐</p>
        </div>
      ) : (
        <div className="playlist-list" ref={listRef}>
          {tracks.map((track, i) => (
            <div
              key={`${track.id}-${i}`}
              className={`track-item ${currentIndex === i ? "active" : ""}`}
              onClick={() => onPlay(i)}
            >
              <span className="track-idx">{i + 1}</span>
              <div className="track-info">
                <span className="track-name">{track.name}</span>
                <span className="track-artist">{track.artist}</span>
              </div>
              <button
                className="track-remove"
                onClick={(e) => {
                  e.stopPropagation();
                  onRemove(i);
                }}
                title="删除"
              >
                ×
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
