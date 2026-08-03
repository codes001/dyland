import { useRef, useState } from "react";
import { useSystemStore } from "../../stores/system-store";
import { usePillStore } from "../../stores/pill-store";
import { ipcClient } from "../../ipc/client";
import { StatusExpandedFallback } from "./StatusExpandedView";

export default function BrightnessExpandedView() {
  const brightness = useSystemStore((s) => s.brightness);
  const clearSelected = useSystemStore((s) => s.clearSelectedStatusFeature);
  const [dragging, setDragging] = useState(false);
  const pendingRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const back = () => {
    clearSelected();
    usePillStore.getState().transitionTo("active");
  };

  if (!brightness || !brightness.has_backlight) return <StatusExpandedFallback feature="brightness" />;

  const percentage = Math.max(0, Math.min(100, brightness.brightness));

  const setBrightness = (next: number) => {
    const clamped = Math.max(0, Math.min(100, next));
    if (pendingRef.current) clearTimeout(pendingRef.current);
    pendingRef.current = setTimeout(() => {
      const cmd = JSON.stringify({ type: "SetBrightness", payload: { brightness: clamped } });
      ipcClient.sendCommand(cmd);
    }, 80);
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
        <span style={{ fontSize: 12, opacity: 0.6 }}>Brightness</span>
      </div>

      <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
        <svg
          width="20"
          height="20"
          viewBox="0 0 24 24"
          fill="none"
          stroke="#ffffff"
          strokeWidth="2"
          strokeLinecap="round"
          style={{ opacity: Math.max(0.3, Math.min(1, percentage / 100)) }}
          aria-hidden
        >
          <circle cx="12" cy="12" r="4.5" fill="#ffffff" stroke="none" />
          <path d="M12 2.5 L12 5 M12 19 L12 21.5 M2.5 12 L5 12 M19 12 L21.5 12 M5.3 5.3 L7 7 M17 17 L18.7 18.7 M18.7 5.3 L17 7 M7 17 L5.3 18.7" stroke="#ffffff" />
        </svg>

        <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 4 }}>
          <input
            type="range"
            min={0}
            max={100}
            value={percentage}
            onChange={(e) => setBrightness(Number(e.target.value))}
            onPointerDown={() => setDragging(true)}
            onPointerUp={() => setDragging(false)}
            style={{ width: "100%", accentColor: "#4ade80" }}
            aria-label="Brightness"
          />
          <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11 }}>
            <span style={{ opacity: 0.5 }}>Brightness</span>
            <span style={{ opacity: 0.9 }}>{percentage}%</span>
          </div>
        </div>
      </div>

      {dragging && (
        <div style={{ fontSize: 11, opacity: 0.5, textAlign: "center" }}>Adjusting…</div>
      )}
    </div>
  );
}
