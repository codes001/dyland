import { useNotificationStore } from "../../stores/notification-store";
import AppIcon from "./AppIcon";
import {
  displayText,
  normalizeUrgency,
  truncateSummary,
} from "../../lib/notification-utils";

export default function PillNotificationContent() {
  const notification = useNotificationStore((s) => s.notification);
  if (!notification) return null;

  const urgency = normalizeUrgency(notification.urgency);
  const isCritical = urgency === "critical";
  const text = truncateSummary(displayText(notification));
  const appLabel = notification.app_name?.trim() || text || "app";

  return (
    <div
      className={isCritical ? "dyd-notification-critical" : undefined}
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
      aria-label={`Notification from ${appLabel}`}
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
        <AppIcon appIcon={notification.app_icon} appName={notification.app_name} size={16} />
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