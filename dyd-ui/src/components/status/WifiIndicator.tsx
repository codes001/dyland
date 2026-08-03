import { useSystemStore } from "../../stores/system-store";

interface WifiIndicatorProps {
  size?: number;
  showSsid?: boolean;
  onClick?: () => void;
}

function signalBars(strength: number): number {
  if (strength <= 25) return 1;
  if (strength <= 50) return 2;
  if (strength <= 75) return 3;
  return 4;
}

export default function WifiIndicator({ size = 16, showSsid = true, onClick }: WifiIndicatorProps) {
  const wifi = useSystemStore((s) => s.wifi);

  if (!wifi) return null;
  if (wifi.state === "none") return null;

  const connected = wifi.state === "connected";
  const connecting = wifi.state === "connecting";
  const bars = signalBars(wifi.strength);

  const icon = (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="currentColor"
      style={{ color: connected ? "#ffffff" : "rgba(255,255,255,0.5)", flexShrink: 0 }}
      role="img"
      aria-label={connected ? `WiFi ${wifi.ssid ?? ""}` : wifi.state}
    >
      {connecting && (
        <>
          <circle cx="12" cy="12" r="1.2" opacity="0.4" />
          <circle cx="12" cy="12" r="4" opacity="0.6" />
          <circle cx="12" cy="12" r="8" opacity="1" />
          <circle cx="12" cy="12" r="11" opacity="0.3" />
        </>
      )}
      {!connecting && (
        <>
          <path d="M12 20 L12 20" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
          {[3, 2, 1].map((level) => (
            <path
              key={level}
              d={
                level === 1
                  ? "M6 12.5 C8.7 9.8 15.3 9.8 18 12.5"
                  : level === 2
                    ? "M8 9.8 C10.4 7.4 13.6 7.4 16 9.8"
                    : "M10 7 C11.3 5.8 12.7 5.8 14 7"
              }
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              opacity={level <= bars ? 1 : 0.22}
            />
          ))}
        </>
      )}
    </svg>
  );

  if (!showSsid) {
    return <span onClick={onClick}>{icon}</span>;
  }

  const label = connected ? (wifi.ssid ? truncate(wifi.ssid, 12) : "WiFi") : wifi.state;

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
      {showSsid && <span style={{ maxWidth: 80, textOverflow: "ellipsis", overflow: "hidden" }}>{label}</span>}
    </span>
  );
}

function truncate(text: string, max: number): string {
  if (text.length <= max) return text;
  return `${text.slice(0, max - 1)}…`;
}
