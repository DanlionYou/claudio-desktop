import type { PlaybackStatus } from "../types";
import "./PlayerControls.css";

interface PlayerControlsProps {
  status: PlaybackStatus;
  progress: { elapsed_secs: number; total_secs: number };
  onPlayPause: () => void;
  onStop: () => void;
  onNext: () => void;
  onPrev: () => void;
  onVolumeChange: (vol: number) => void;
}

function formatTime(secs: number): string {
  if (secs <= 0 || !isFinite(secs)) return "0:00";
  const m = Math.floor(secs / 60);
  const s = Math.floor(secs % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}

const VIZ_BARS = [0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7];

export function PlayerControls({
  status,
  progress,
  onPlayPause,
  onStop,
  onNext,
  onPrev,
  onVolumeChange,
}: PlayerControlsProps) {
  const isPlaying = status.state === "playing";
  const track = status.current_track;
  const hasTrack = track !== null;
  const elapsed = progress.elapsed_secs;
  const total = progress.total_secs || 1;
  const progressPct = Math.min((elapsed / total) * 100, 100);

  return (
    <div className={`player-controls ${isPlaying ? "playing" : "paused"}`}>
      {/* Main row */}
      <div className="player-main-row">
        {/* Left: Track info + Visualizer */}
        <div className="left-group">
          <div className="track-display">
            <span className="track-display-name">
              {track ? track.name : "✦ 未播放"}
            </span>
            <span className="track-display-artist">
              {track ? track.artist : "点击 + 添加音乐文件"}
            </span>
          </div>
          <div className={`visualizer ${isPlaying ? "active" : ""}`}>
            {VIZ_BARS.map((delay, i) => (
              <div
                key={i}
                className="viz-bar"
                style={{ animationDelay: `${delay}s` }}
              />
            ))}
          </div>
        </div>

        {/* Center: Controls */}
        <div className="control-buttons">
          <button className="ctrl-btn ctrl-prev" onClick={onPrev} disabled={!hasTrack} title="上一曲">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <path d="M6 6h2v12H6zm3.5 6l8.5 6V6z"/>
            </svg>
          </button>
          <button className="ctrl-btn ctrl-btn-play" onClick={onPlayPause} disabled={!hasTrack} title={isPlaying ? "暂停" : "播放"}>
            {isPlaying ? (
              <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
                <path d="M6 19h4V5H6zm8-14v14h4V5z"/>
              </svg>
            ) : (
              <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
                <path d="M8 5v14l11-7z"/>
              </svg>
            )}
          </button>
          <button className="ctrl-btn" onClick={onStop} disabled={!hasTrack} title="停止">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <path d="M6 6h12v12H6z"/>
            </svg>
          </button>
          <button className="ctrl-btn ctrl-next" onClick={onNext} disabled={!hasTrack} title="下一曲">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <path d="M6 18l8.5-6L6 6zm12-12v12h-2V6z"/>
            </svg>
          </button>
        </div>

        {/* Right: Volume */}
        <div className="volume-section">
          <span className="vol-icon">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/>
              {status.volume > 0 && (
                <>
                  <path d="M15.54 8.46a5 5 0 010 7.07"/>
                  {status.volume > 0.5 && (
                    <path d="M19.07 4.93a10 10 0 010 14.14"/>
                  )}
                </>
              )}
            </svg>
          </span>
          <input
            type="range"
            className="vol-slider"
            min={0}
            max={1}
            step={0.01}
            value={status.volume}
            onChange={(e) => onVolumeChange(Number(e.target.value))}
            style={{
              background: `linear-gradient(90deg, var(--slider-fill) ${status.volume * 100}%, var(--slider-track) ${status.volume * 100}%)`,
            }}
          />
          <span className="vol-value">{Math.round(status.volume * 100)}</span>
        </div>
      </div>

      {/* Progress bar */}
      <div className="progress-section">
        <span className="time-label">{formatTime(elapsed)}</span>
        <div className="progress-track">
          <div className="progress-fill" style={{ width: `${progressPct}%` }}>
            <div className="progress-thumb" />
          </div>
        </div>
        <span className="time-label">{formatTime(total)}</span>
      </div>
    </div>
  );
}
