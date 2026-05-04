import { useState, useRef, useCallback, useEffect } from "react";
import { Header } from "./components/Header";
import { DigitalClock } from "./components/DigitalClock";
import { PlayerControls } from "./components/PlayerControls";
import { Playlist } from "./components/Playlist";
import { ChatBox } from "./components/ChatBox";
import { useTheme } from "./hooks/useTheme";
import { usePlayer } from "./hooks/usePlayer";
import "./App.css";

const CHAT_MIN = 100;
const CHAT_DEFAULT = 150;

function App() {
  const { theme, toggleTheme } = useTheme();
  const {
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
  } = usePlayer();

  const [chatHeight, setChatHeight] = useState(CHAT_DEFAULT);
  const dragState = useRef<{ startY: number; startHeight: number } | null>(null);
  const [isDragging, setIsDragging] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);
  const chatHeightRef = useRef(CHAT_DEFAULT);

  // Dynamic max: 90% of content-panel height, uncapped
  const getChatMax = useCallback(() => {
    const h = panelRef.current?.clientHeight ?? 500;
    return Math.max(CHAT_MIN, Math.floor(h * 0.9));
  }, []);

  // Re-clamp on resize (fullscreen toggle, window resize, etc.)
  useEffect(() => {
    const el = panelRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => {
      setChatHeight((prev) => {
        const max = getChatMax();
        return prev > max ? max : prev;
      });
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, [getChatMax]);

  const handleFilesAdded = async (paths: string[]) => {
    if (paths.length > 0) {
      await addFiles(paths);
    }
  };

  const handleDragStart = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      const max = getChatMax();
      dragState.current = {
        startY: e.clientY,
        startHeight: Math.min(chatHeightRef.current, max),
      };
      setIsDragging(true);

      const onMove = (e: MouseEvent) => {
        if (!dragState.current) return;
        const delta = dragState.current.startY - e.clientY;
        const currentMax = getChatMax();
        const newHeight = Math.max(
          CHAT_MIN,
          Math.min(currentMax, dragState.current.startHeight + delta)
        );
        setChatHeight(newHeight);
        chatHeightRef.current = newHeight;
      };

      const onUp = () => {
        dragState.current = null;
        setIsDragging(false);
        document.removeEventListener("mousemove", onMove);
        document.removeEventListener("mouseup", onUp);
      };

      document.addEventListener("mousemove", onMove);
      document.addEventListener("mouseup", onUp);
    },
    [getChatMax]
  );

  // Prevent text selection while dragging
  useEffect(() => {
    if (isDragging) {
      document.body.style.userSelect = "none";
      document.body.style.cursor = "row-resize";
    } else {
      document.body.style.userSelect = "";
      document.body.style.cursor = "";
    }
  }, [isDragging]);

  const currentMax = getChatMax();

  return (
    <div className="app-container">
      <Header theme={theme} onToggleTheme={toggleTheme} />
      <DigitalClock playbackState={status.state} />
      <PlayerControls
        status={status}
        progress={progress}
        onPlayPause={togglePlayPause}
        onStop={stop}
        onNext={next}
        onPrev={prev}
        onVolumeChange={setVolume}
      />
      <div className="content-panel" ref={panelRef}>
        <div className="playlist-wrapper">
          <Playlist
            tracks={playlist}
            currentIndex={status.current_track_index}
            onPlay={playTrack}
            onRemove={removeTrack}
            onFilesAdded={handleFilesAdded}
          />
        </div>
        <div
          className={`drag-handle ${isDragging ? "active" : ""}`}
          onMouseDown={handleDragStart}
        />
        <div
          className="chatbox-wrapper"
          style={{ height: Math.min(chatHeight, currentMax), minHeight: CHAT_MIN }}
        >
          <ChatBox />
        </div>
      </div>
    </div>
  );
}

export default App;
