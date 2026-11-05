import { useEffect, useState } from "react";

/** Always 24-hour local time. The shell hides the panel clock, so the idle island is the clock. */
export function formatIslandTime(date: Date): string {
  const hours = String(date.getHours()).padStart(2, "0");
  const minutes = String(date.getMinutes()).padStart(2, "0");
  return `${hours}:${minutes}`;
}

export default function IdleClock() {
  const [now, setNow] = useState(() => new Date());

  useEffect(() => {
    const id = window.setInterval(() => setNow(new Date()), 1000);
    return () => window.clearInterval(id);
  }, []);

  const label = formatIslandTime(now);

  return (
    <span
      role="timer"
      aria-label={label}
      style={{
        color: "var(--pill-text)",
        fontSize: 15,
        fontWeight: 600,
        letterSpacing: "0.04em",
        fontVariantNumeric: "tabular-nums",
        lineHeight: 1,
        userSelect: "none",
      }}
    >
      {label}
    </span>
  );
}
