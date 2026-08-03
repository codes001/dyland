import { useRef, useState } from "react";
import { useSystemStore } from "../../stores/system-store";
import { usePillStore } from "../../stores/pill-store";
import { ipcClient } from "../../ipc/client";
import { StatusExpandedFallback } from "./StatusExpandedView";

export default function VolumeExpandedView() {
  const volume = useSystemStore((s) => s.volume);
  const clearSelected = useSystemStore((s) => s.clearSelectedStatusFeature);
  const [dragging, setDragging] = useState(false);
  const pendingRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const back = () => {
    clearSelected();
    usePillStore.getState().transitionTo("active");
  };

  if (!volume) return <StatusExpandedFallback feature="volume" />;

  const level = Math.max(0, Math.min(1, volume.volume));
  const muted = volume.muted || level === 0;
  const percentage = Math.round(level * 100);

  const setVolume = (next: number) => {
    const clamped = Math.max(0, Math.min(1, next));
    if (pendingRef.current) clearTimeout(pendingRef.current);
    pendingRef.current = setTimeout(() => {
      const cmd = JSON.stringify({ type: "SetVolume", payload: { volume: clamped } });
      ipcClient.sendCommand(cmd);
    }, 80);
  };

  const toggleMute = () => {
    const cmd = JSON.stringify({ type: "SetMute", payload: { muted: !muted } });
    ipcClient.sendCommand(cmd);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12, width: "100%" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <button
          onClick={back}
          style={{ background: "none", border: "none", color: "#ffffff", opacity: 0.6, fontSize: 12, cursor: "pointer" }}
          aria-label="Back to status"
        >
          ←
        </button>
        <span style={{ fontSize: 12, opacity: 0.6 }}>Volume</span>
      </div>

      <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
        <button
          onClick={toggleMute}
          style={{ background: "none", border: "none", color: "#ffffff", cursor: "pointer", opacity: muted ? 0.5 : 1 }}
          aria-label={muted ? "Unmute" : "Mute"}
        >
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round">
            <path d="M3 9 L6 9 L10 5 L10 19 L6 15 L3 15 Z" fill="currentColor" opacity="0.9" />
            {muted ? (
              <path d="M16 8 L22 14 M22 8 L16 14" stroke="currentColor" />
            ) : (
              <>
                <path d="M14 9.5 C15.3 10.8 15.3 13.2 14 14.5" stroke="currentColor" />
                <path d="M16.5 6.5 C19 8.5 19 15.5 16.5 17.5" stroke="currentColor" />
                <path d="M19 3.5 C23 6.5 23 17.5 19 20.5" stroke="currentColor" />
              </>
            )}
          </svg>
        </button>

        <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 4 }}>
          <input
            type="range"
            min={0}
            max={100}
            value={percentage}
            onChange={(e) => setVolume(Number(e.target.value) / 100)}
            onPointerDown={() => setDragging(true)}
            onPointerUp={() => setDragging(false)}
            style={{ width: "100%", accentColor: "#4ade80" }}
            aria-label="Volume"
          />
          <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11 }}>
            <span style={{ opacity: 0.5 }}>{muted ? "Muted" : "Volume"}</span>
            <span style={{ opacity: 0.9 }}>{percentage}%</span>
          </div>
        </div>
      </div>

      {!dragging && volume.sink_name && (
        <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12 }}>
          <span style={{ opacity: 0.6 }}>Output</span>
          <span style={{ maxWidth: 180, textOverflow: "ellipsis", overflow: "hidden", whiteSpace: "nowrap" }}>
            {volume.sink_name}
          </span>
        </div>
      )}
    </div>
  );
}
