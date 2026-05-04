import { useClock } from "../hooks/useClock";
import type { PlaybackState } from "../types";
import "./DigitalClock.css";

interface DigitalClockProps {
  playbackState: PlaybackState;
}

export function DigitalClock({ playbackState }: DigitalClockProps) {
  const { hh, mm, dayName, day, monthName, year } = useClock();
  const isOnAir = playbackState === "playing";

  return (
    <div className="digital-clock">
      <div className="clock-time">
        <span className="clock-hh">{hh}</span>
        <span className="clock-separator">:</span>
        <span className="clock-mm">{mm}</span>
      </div>
      <div className="clock-date">
        {dayName}, {day} {monthName} {year}
      </div>
      <div className={`clock-status ${isOnAir ? "on-air" : "offline"}`}>
        <span className="status-dot" />
        <span className="status-text">{isOnAir ? "ON AIR" : "OFFLINE"}</span>
      </div>
    </div>
  );
}
