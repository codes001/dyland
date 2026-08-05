import { useState, useEffect } from "react";

interface Props {
  actionLabel: string;
  value: string;
  onChange: (keyCombo: string) => void;
}

export default function ShortcutRecorder({ actionLabel, value, onChange }: Props) {
  const [recording, setRecording] = useState(false);

  useEffect(() => {
    if (!recording) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      if (e.key === "Escape") {
        setRecording(false);
        return;
      }

      const parts: string[] = [];
      if (e.ctrlKey) parts.push("Ctrl");
      if (e.altKey) parts.push("Alt");
      if (e.shiftKey) parts.push("Shift");
      if (e.metaKey) parts.push("Super");

      const keyName =
        e.key === " "
          ? "Space"
          : e.key.length === 1
          ? e.key.toUpperCase()
          : e.key;

      if (!["Control", "Alt", "Shift", "Meta"].includes(e.key)) {
        parts.push(keyName);
        const combo = parts.join("+");
        onChange(combo);
        setRecording(false);
      }
    };

    window.addEventListener("keydown", handleKeyDown, { capture: true });
    return () => window.removeEventListener("keydown", handleKeyDown, { capture: true });
  }, [recording, onChange]);

  return (
    <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", padding: "6px 0" }}>
      <span style={{ fontSize: 13, color: "var(--pill-text)" }}>{actionLabel}</span>
      <button
        onClick={() => setRecording(true)}
        aria-label={`Configure shortcut for ${actionLabel}`}
        style={{
          padding: "4px 10px",
          borderRadius: 6,
          border: recording ? "1px solid #3b82f6" : "1px solid var(--pill-border)",
          background: recording ? "rgba(59, 130, 246, 0.2)" : "rgba(255, 255, 255, 0.05)",
          color: "var(--pill-text)",
          fontSize: 12,
          fontFamily: "monospace",
          cursor: "pointer",
        }}
      >
        {recording ? "Press key combination..." : value || "Unassigned"}
      </button>
    </div>
  );
}
