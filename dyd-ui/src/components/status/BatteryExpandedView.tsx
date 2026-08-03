import { useSystemStore } from "../../stores/system-store";
import { usePillStore } from "../../stores/pill-store";
import { StatusExpandedFallback } from "./StatusExpandedView";

function formatDuration(seconds: number | null): string | null {
  if (seconds === null || seconds === undefined || seconds <= 0) return null;
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours === 0) return `${minutes}m`;
  return `${hours}h ${minutes}m`;
}

function formatBatteryTime(battery: {
  state: string;
  time_to_empty: number | null;
  time_to_full: number | null;
  percentage: number;
}): string | null {
  if (battery.state === "charging") {
    if (battery.percentage >= 100) return "Fully Charged";
    const t = formatDuration(battery.time_to_full);
    return t ? `${t} until full` : "Estimating…";
  }
  if (battery.state === "discharging") {
    const t = formatDuration(battery.time_to_empty);
    return t ? `${t} remaining` : "Estimating…";
  }
  if (battery.state === "full") return "Fully Charged";
  return "Estimating…";
}

export default function BatteryExpandedView() {
  const battery = useSystemStore((s) => s.battery);
  const clearSelected = useSystemStore((s) => s.clearSelectedStatusFeature);

  const back = () => {
    clearSelected();
    usePillStore.getState().transitionTo("active");
  };

  if (!battery || !battery.is_present) return <StatusExpandedFallback feature="battery" />;

  const percentage = Math.round(battery.percentage);
  const state = battery.state;
  const isCharging = state === "charging" || state === "pending-charge";
  const isFull = state === "full";
  const time = formatBatteryTime(battery);
  const health = battery.health != null ? Math.round(battery.health) : null;

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
        <span style={{ fontSize: 12, opacity: 0.6, textTransform: "capitalize" }}>
          {isCharging ? "Charging" : state}
        </span>
      </div>

      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", gap: 16 }}>
        <svg width="72" height="40" viewBox="0 0 32 18" fill="none">
          <rect x="1" y="1" width="28" height="16" rx="3" stroke="#ffffff" strokeWidth="1.5" opacity="0.9" />
          <rect x="30" y="5.5" width="2" height="7" rx="1" fill="#ffffff" />
          <clipPath id="dyd-battery-exp-clip">
            <rect x="3" y="3" width="24" height="12" rx="1.5" />
          </clipPath>
          <rect
            x="3"
            y="3"
            width={24 * Math.max(0, Math.min(1, percentage / 100))}
            height="12"
            rx="1.5"
            fill={isCharging ? "#4ade80" : "#ffffff"}
            clipPath="url(#dyd-battery-exp-clip)"
            opacity={isCharging ? 0.4 : 0.9}
          />
          {isCharging && <path d="M13 4 L8 10 L12 10 L10 14 L16 8 L12 8 Z" fill="#4ade80" />}
        </svg>
        <div style={{ display: "flex", flexDirection: "column" }}>
          <span style={{ fontSize: 32, fontWeight: 700, lineHeight: 1 }}>{percentage}%</span>
          <span style={{ fontSize: 12, opacity: 0.6 }}>{time}</span>
        </div>
      </div>

      <div style={{ display: "flex", gap: 8, justifyContent: "center", flexWrap: "wrap" }}>
        {health != null && (
          <span style={{ fontSize: 11, opacity: 0.7, background: "rgba(255,255,255,0.08)", borderRadius: 999, padding: "4px 10px" }}>
            Health {health}%
          </span>
        )}
        {isFull && (
          <span style={{ fontSize: 11, opacity: 0.7, background: "rgba(74,222,128,0.15)", color: "#4ade80", borderRadius: 999, padding: "4px 10px" }}>
            Fully Charged
          </span>
        )}
      </div>
    </div>
  );
}
