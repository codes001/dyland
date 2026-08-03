import { useSystemStore } from "../../stores/system-store";

interface BrightnessIndicatorProps {
  size?: number;
  showPercentage?: boolean;
  onClick?: () => void;
}

export default function BrightnessIndicator({ size = 16, showPercentage = true, onClick }: BrightnessIndicatorProps) {
  const brightness = useSystemStore((s) => s.brightness);

  if (!brightness || !brightness.has_backlight) return null;

  const percentage = brightness.brightness;
  const opacity = Math.max(0.25, Math.min(1, percentage / 100));

  const icon = (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      style={{ color: "#ffffff", opacity, flexShrink: 0 }}
      role="img"
      aria-label={`Brightness ${percentage}%`}
    >
      <circle cx="12" cy="12" r="4.5" fill="currentColor" stroke="none" />
      <path d="M12 2.5 L12 5 M12 19 L12 21.5 M2.5 12 L5 12 M19 12 L21.5 12 M5.3 5.3 L7 7 M17 17 L18.7 18.7 M18.7 5.3 L17 7 M7 17 L5.3 18.7" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
    </svg>
  );

  if (!showPercentage || size < 20) {
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
      <span style={{ opacity: 0.9 }}>{percentage}%</span>
    </span>
  );
}
