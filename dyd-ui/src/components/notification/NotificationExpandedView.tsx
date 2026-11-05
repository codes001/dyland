import { useEffect, useRef, useState } from "react";
import { useNotificationStore } from "../../stores/notification-store";
import { ipcClient } from "../../ipc/client";
import AppIcon from "./AppIcon";
import { displayText, normalizeUrgency, relativeTime } from "../../lib/notification-utils";
import CallControls, { useCallElapsed } from "./CallControls";

const MAX_VISIBLE_ACTIONS = 3;
const MAX_BODY_LINES = 4;

function DismissButton({ onClick }: { onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      aria-label="Dismiss notification"
      style={{
        position: "absolute",
        top: 0,
        right: 0,
        width: 24,
        height: 24,
        borderRadius: 12,
        border: "none",
        background: "rgba(255,255,255,0.08)",
        color: "rgba(255,255,255,0.7)",
        cursor: "pointer",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
      }}
    >
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
        <path d="M18 6 6 18M6 6l12 12" />
      </svg>
    </button>
  );
}

export default function NotificationExpandedView() {
  const call = useNotificationStore((s) => s.call);
  const notification = useNotificationStore((s) => s.call ?? s.notification);
  const [overflowOpen, setOverflowOpen] = useState(false);
  const [sentActions, setSentActions] = useState<Set<string>>(new Set());
  const sentActionsRef = useRef<Set<string>>(new Set());

  const notificationId = notification?.id;
  const isCall = notification?.urgency === "call";
  const answered = call?.phase === "active";
  const elapsed = useCallElapsed(answered ? call?.started_at : undefined);
  useEffect(() => {
    setOverflowOpen(false);
    setSentActions(new Set());
    sentActionsRef.current = new Set();
  }, [notificationId]);

  if (!notification) return null;

  const urgency = normalizeUrgency(notification.urgency);
  const hasBody = Boolean(notification.body?.trim());
  const actions = notification.actions ?? [];
  const visible = actions.slice(0, MAX_VISIBLE_ACTIONS);
  const overflow = actions.slice(MAX_VISIBLE_ACTIONS);
  const timestamp = relativeTime(notification.timestamp);
  const appLabel = notification.app_name?.trim() || displayText(notification) || "App";

  const close = () => {
    if (notification.urgency === "call") {
      useNotificationStore.getState().clearCall();
      return;
    }
    const cmd = JSON.stringify({ type: "NotificationClose", payload: { id: notification.id } });
    ipcClient.sendCommand(cmd);
    useNotificationStore.getState().clearNotification();
  };

  const invoke = (key: string) => {
    if (sentActionsRef.current.has(key)) return;
    sentActionsRef.current = new Set(sentActionsRef.current).add(key);
    setSentActions(sentActionsRef.current);
    const cmd = JSON.stringify({ type: "NotificationAction", payload: { id: notification.id, action_key: key } });
    ipcClient.sendCommand(cmd);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 10, width: "100%", position: "relative" }}>
      <DismissButton onClick={close} />

      <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
        <AppIcon appIcon={notification.app_icon} appName={notification.app_name} size={32} />
        <div style={{ display: "flex", flexDirection: "column", minWidth: 0, flex: 1 }}>
          <span
            style={{
              fontSize: 13,
              fontWeight: 600,
              lineHeight: 1.3,
              color: "#fff",
              whiteSpace: "nowrap",
              overflow: "hidden",
              textOverflow: "ellipsis",
            }}
          >
            {appLabel}
          </span>
          {timestamp && (
            <span style={{ fontSize: 11, lineHeight: 1.2, color: "rgba(255,255,255,0.5)" }}>
              {timestamp}
            </span>
          )}
        </div>
      </div>

      {answered ? (
        <span
          style={{
            fontSize: 28,
            fontWeight: 600,
            lineHeight: 1.1,
            color: "#30D158",
            fontVariantNumeric: "tabular-nums",
          }}
        >
          {elapsed || "0:00"}
        </span>
      ) : hasBody ? (
        <div
          style={{
            maxHeight: `calc(${MAX_BODY_LINES} * 1.4em)`,
            overflowY: "auto",
          }}
        >
          <p
            style={{
              fontSize: 14,
              lineHeight: 1.4,
              color: "rgba(255,255,255,0.9)",
              margin: 0,
              wordBreak: "break-word",
              whiteSpace: "pre-wrap",
            }}
          >
            {notification.body}
          </p>
        </div>
      ) : (
        <span style={{ fontSize: 16, fontWeight: 600, lineHeight: 1.3, color: "#fff" }}>
          {answered ? elapsed || "0:00" : displayText(notification) || "Notification"}
        </span>
      )}

      {isCall && <CallControls answered={answered} />}

      {!isCall && actions.length > 0 && (
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
          {visible.map((action) => (
            <button
              key={action.key}
              onClick={() => invoke(action.key)}
              disabled={sentActions.has(action.key)}
              style={{
                padding: "6px 14px",
                borderRadius: 16,
                border: "1px solid rgba(255,255,255,0.18)",
                background: "rgba(255,255,255,0.06)",
                color: "#fff",
                fontSize: 13,
                fontWeight: 500,
                cursor: "pointer",
                maxWidth: 160,
                whiteSpace: "nowrap",
                overflow: "hidden",
                textOverflow: "ellipsis",
              }}
            >
              {action.label}
            </button>
          ))}
          {overflow.length > 0 && (
            <>
              <button
                onClick={() => setOverflowOpen((v) => !v)}
                style={{
                  padding: "6px 10px",
                  borderRadius: 16,
                  border: "1px solid rgba(255,255,255,0.18)",
                  background: "rgba(255,255,255,0.06)",
                  color: "rgba(255,255,255,0.8)",
                  fontSize: 13,
                  cursor: "pointer",
                }}
              >
                +{overflow.length}
              </button>
              {overflowOpen && (
                <div style={{ display: "flex", gap: 8, flexWrap: "wrap", width: "100%" }}>
                  {overflow.map((action) => (
                    <button
                      key={action.key}
                      onClick={() => invoke(action.key)}
                      disabled={sentActions.has(action.key)}
                      style={{
                        padding: "6px 14px",
                        borderRadius: 16,
                        border: "1px solid rgba(255,255,255,0.18)",
                        background: "rgba(255,255,255,0.06)",
                        color: "#fff",
                        fontSize: 13,
                        cursor: "pointer",
                        maxWidth: 160,
                        whiteSpace: "nowrap",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                      }}
                    >
                      {action.label}
                    </button>
                  ))}
                </div>
              )}
            </>
          )}
        </div>
      )}

      {urgency === "critical" && (
        <span style={{ color: "#ff3b30", fontSize: 11, fontWeight: 600 }}>Critical</span>
      )}
    </div>
  );
}