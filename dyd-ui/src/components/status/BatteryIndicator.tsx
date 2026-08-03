import { useSystemStore } from "../../stores/system-store";

interface BatteryIndicatorProps {
  size?: number;
  showPercentage?: boolean;
  onClick?: () => void;
}

function getBatteryColor(percentage: number, state: string): string {
  if (state === "charging") return "#4ade80";
  if (percentage < 20) return "#ef4444";
  return "#ffffff";
}

export default function BatteryIndicator({ size = 16, showPercentage = true, onClick }: BatteryIndicatorProps) {
  const battery = useSystemStore((s) => s.battery);

  if (!battery || !battery.is_present) return null;

  const percentage = Math.round(battery.percentage);
  const state = battery.state;
  const isCharging = state === "charging" || state === "pending-charge";
  const isFull = state === "full" || percentage >= 100;
  const isCritical = percentage < 10 && !isCharging;
  const color = getBatteryColor(percentage, state);

  const level = Math.max(0, Math.min(1, percentage / 100));

  const icon = (
    <svg
      width={size}
      height={size * 0.55}
      viewBox="0 0 32 18"
      fill="none"
      style={{
        color,
        opacity: isCritical ? 0.85 : 1,
        animation: isCritical ? "dyd-battery-pulse 1.2s ease-in-out infinite" : undefined,
        flexShrink: 0,
      }}
      role="img"
      aria-label={`Battery ${percentage}% ${state}`}
    >
      <rect x="1" y="1" width="28" height="16" rx="3" stroke="currentColor" strokeWidth="2" />
      <rect x="30" y="5.5" width="2" height="7" rx="1" fill="currentColor" />
      <clipPath id="dyd-battery-clip">
        <rect x="3" y="3" width="24" height="12" rx="1.5" />
      </clipPath>
      <rect
        x="3"
        y="3"
        width={24 * level}
        height="12"
        rx="1.5"
        fill={isFull && !isCharging ? "currentColor" : color}
        clipPath="url(#dyd-battery-clip)"
        opacity={isCharging ? 0.4 : 0.9}
      />
      {isCharging && (
        <path
          d="M13 4 L8 10 L12 10 L10 14 L16 8 L12 8 Z"
          fill={color}
        />
      )}
      {isFull && !isCharging && (
        <path d="M12 13 L16 9 L17 10 L12 14 Z" fill="#111111" />
      )}
    </svg>
  );

  if (!showPercentage) {
    return <span onClick={onClick}>{icon}</span>;
  }

  return (
    <span
      onClick={onClick}
      style={{
        display: "flex",
        alignItems: "center",
        gap: 4,
        color: "#ffffff",
        fontSize: 11,
        fontWeight: 600,
        whiteSpace: "nowrap",
      }}
    >
      {icon}
      <span style={{ color: isCritical ? "#ef4444" : undefined }}>{percentage}%</span>
    </span>
  );
}
