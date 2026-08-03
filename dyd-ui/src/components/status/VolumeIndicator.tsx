import { useSystemStore } from "../../stores/system-store";

interface VolumeIndicatorProps {
  size?: number;
  showPercentage?: boolean;
  onClick?: () => void;
}

function speakerPath(muted: boolean, level: number) {
  if (muted || level === 0) {
    return (
      <>
        <path d="M3 9 L6 9 L10 5 L10 19 L6 15 L3 15 Z" stroke="currentColor" strokeWidth="1.8" strokeLinejoin="round" fill="currentColor" opacity="0.9" />
        <path d="M16 8 L22 14 M22 8 L16 14" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
      </>
    );
  }
  const waves = level < 34 ? 1 : level < 67 ? 2 : 3;
  return (
    <>
      <path d="M3 9 L6 9 L10 5 L10 19 L6 15 L3 15 Z" stroke="currentColor" strokeWidth="1.8" strokeLinejoin="round" fill="currentColor" opacity="0.9" />
      {waves >= 1 && <path d="M14 9.5 C15.3 10.8 15.3 13.2 14 14.5" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />}
      {waves >= 2 && <path d="M16.5 6.5 C19 8.5 19 15.5 16.5 17.5" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />}
      {waves >= 3 && <path d="M19 3.5 C23 6.5 23 17.5 19 20.5" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />}
    </>
  );
}

export default function VolumeIndicator({ size = 16, showPercentage = true, onClick }: VolumeIndicatorProps) {
  const volume = useSystemStore((s) => s.volume);

  if (!volume) return null;

  const level = Math.max(0, Math.min(1, volume.volume));
  const muted = volume.muted || level === 0;
  const percentage = Math.round(level * 100);

  const icon = (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      style={{ color: muted ? "rgba(255,255,255,0.5)" : "#ffffff", flexShrink: 0 }}
      role="img"
      aria-label={muted ? "Volume muted" : `Volume ${percentage}%`}
    >
      {speakerPath(muted, level)}
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
        fontWeight: 500,
        whiteSpace: "nowrap",
      }}
    >
      {icon}
      <span style={{ opacity: muted ? 0.6 : 1 }}>{percentage}%</span>
    </span>
  );
}
