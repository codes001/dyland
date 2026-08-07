import { useActivityStore } from "../stores/activity-store";

export default function PillActivityContent() {
  const activity = useActivityStore((s) => s.currentActivity);
  if (!activity) return null;

  const content = (activity.content ?? {}) as {
    text?: string;
    severity?: string;
  };
  const isError = content.severity === "error" || content.severity === "critical";
  const text = content.text?.trim() || activity.source;

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "flex-start",
        gap: 8,
        width: "100%",
        maxWidth: 200,
        position: "relative",
      }}
      role="status"
      aria-label={`Activity from ${activity.source}`}
    >
      <svg
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke={isError ? "#ff3b30" : "rgba(255,255,255,0.85)"}
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        style={{ flexShrink: 0 }}
        role="img"
        aria-label="terminal"
      >
        <polyline points="4 17 10 11 4 5" />
        <line x1="12" y1="19" x2="20" y2="19" />
      </svg>
      <span
        style={{
          color: "#fff",
          fontSize: 13,
          fontWeight: 500,
          lineHeight: 1.3,
          whiteSpace: "nowrap",
          overflow: "hidden",
          textOverflow: "ellipsis",
          flex: 1,
          minWidth: 0,
        }}
      >
        {text || "Activity"}
      </span>
      {isError && (
        <span
          role="img"
          aria-label="error"
          style={{
            width: 7,
            height: 7,
            borderRadius: "50%",
            backgroundColor: "#ff3b30",
            flexShrink: 0,
          }}
        />
      )}
    </div>
  );
}
