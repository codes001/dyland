import { useNotificationStore } from "../../stores/notification-store";
import AppIcon from "./AppIcon";
import CallControls, { useCallElapsed } from "./CallControls";
import {
  displayText,
  normalizeUrgency,
  truncateSummary,
} from "../../lib/notification-utils";

export default function PillNotificationContent() {
  const call = useNotificationStore((s) => s.call);
  const notification = useNotificationStore((s) => s.call ?? s.notification);
  const isCall = notification?.urgency === "call";
  const answered = call?.phase === "active";
  const elapsed = useCallElapsed(answered ? call?.started_at : undefined);
  if (!notification) return null;
  const urgency = normalizeUrgency(notification.urgency);
  const isCritical = urgency === "critical";
  const text = truncateSummary(isCall ? callLabel(notification.summary, answered) : displayText(notification));
  const appLabel = notification.app_name?.trim() || text || "app";
  const timer = elapsed || "0:00";

  return (
    <div
      className={isCritical ? "dyd-notification-critical" : undefined}
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "flex-start",
        gap: 8,
        width: "100%",
        maxWidth: isCall ? "none" : 200,
        position: "relative",
      }}
      role="status"
      aria-label={
        answered
          ? `On call with ${text || appLabel}, ${timer}`
          : isCall
            ? `Incoming call from ${appLabel}`
            : `Notification from ${appLabel}`
      }
    >
      <div
        style={{
          width: 20,
          height: 20,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          flexShrink: 0,
        }}
      >
        {isCall ? <CallGlyph /> : <AppIcon appIcon={notification.app_icon} appName={notification.app_name} size={16} />}
      </div>
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
        {text || "Notification"}
      </span>
      {answered && (
        <span
          style={{
            color: "#30D158",
            fontSize: 14,
            fontWeight: 700,
            fontVariantNumeric: "tabular-nums",
            flexShrink: 0,
          }}
        >
          {timer}
        </span>
      )}
      {isCall && <CallControls compact answered={answered} />}
      {isCritical && (
        <span
          role="img"
          aria-label="critical"
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

function callLabel(summary: string | null | undefined, answered: boolean): string {
  const text = summary?.trim() || "";
  if (!answered) return text || "Incoming call";
  const fromCaller = text.replace(/^incoming\s+(?:voice\s+|video\s+)?call\s+from\s+/i, "");
  if (fromCaller !== text && fromCaller.trim()) return fromCaller.trim();
  const stripped = text.replace(/^incoming\s+/i, "").trim();
  return stripped || "On call";
}

function CallGlyph() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <path
        d="M7 3.5h2.2c.4 0 .8.3.9.7l.8 2.4a1 1 0 0 1-.3 1L8.8 9.2a12.5 12.5 0 0 0 6 6l1.6-1.8a1 1 0 0 1 1-.3l2.4.8c.4.1.7.5.7.9V17a2 2 0 0 1-2.2 2A15.5 15.5 0 0 1 5 8.7 2 2 0 0 1 7 3.5Z"
        fill="#fff"
      />
    </svg>
  );
}