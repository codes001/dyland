import { useSystemStore } from "../../stores/system-store";

interface BluetoothIndicatorProps {
  size?: number;
  showCount?: boolean;
  onClick?: () => void;
}

export default function BluetoothIndicator({ size = 16, showCount = true, onClick }: BluetoothIndicatorProps) {
  const bluetooth = useSystemStore((s) => s.bluetooth);

  if (!bluetooth || bluetooth.present === false) return null;

  const connectedCount = bluetooth.devices.filter((d) => d.connected).length;
  const on = bluetooth.powered;

  const icon = (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      style={{
        color: on ? "#ffffff" : "rgba(255,255,255,0.5)",
        opacity: on ? 1 : 0.5,
        flexShrink: 0,
      }}
      role="img"
      aria-label={on ? "Bluetooth on" : "Bluetooth off"}
    >
      <polyline points="6.5 6.5 17.5 17.5 12 22 12 2 17.5 6.5 6.5 17.5" />
    </svg>
  );

  if (!showCount || !on || connectedCount === 0) {
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
        fontWeight: 500,
        whiteSpace: "nowrap",
      }}
    >
      {icon}
      <span>{connectedCount}</span>
    </span>
  );
}
