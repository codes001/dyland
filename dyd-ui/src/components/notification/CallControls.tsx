import { useEffect, useRef, useState } from "react";
import { ipcClient } from "../../ipc/client";

type CallAction = "accept" | "reject";

export function formatCallElapsed(startedAt: number, now = Date.now()): string {
  const total = Math.max(0, Math.floor(now / 1000 - startedAt));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;
  const clock = `${minutes}:${String(seconds).padStart(2, "0")}`;
  return hours > 0 ? `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}` : clock;
}

export function useCallElapsed(startedAt?: number): string {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!startedAt) return;
    const id = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(id);
  }, [startedAt]);
  if (!startedAt) return "";
  return formatCallElapsed(startedAt, now);
}

function sendCall(action: CallAction) {
  void ipcClient
    .sendCommand(JSON.stringify({ type: "CallControl", payload: { action } }))
    .catch(() => {});
}

function stopGesture(event: React.PointerEvent) {
  event.stopPropagation();
}

export default function CallControls({
  compact = false,
  answered = false,
}: {
  compact?: boolean;
  answered?: boolean;
}) {
  const lastSent = useRef(0);
  const size = compact ? 26 : 36;

  const press = (action: CallAction) => {
    const now = Date.now();
    if (now - lastSent.current < 700) return;
    lastSent.current = now;
    sendCall(action);
  };

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: compact ? 6 : 10,
        flexShrink: 0,
      }}
    >
      <CallButton
        label={answered ? "End call" : "Decline call"}
        compact={compact}
        size={size}
        background="#FF453A"
        onPress={() => press("reject")}
      >
        <PhoneIcon down />
        {!compact && <span>{answered ? "End" : "Decline"}</span>}
      </CallButton>
      {!answered && (
        <CallButton
          label="Accept call"
          compact={compact}
          size={size}
          background="#30D158"
          onPress={() => press("accept")}
        >
          <PhoneIcon />
          {!compact && <span>Accept</span>}
        </CallButton>
      )}
    </div>
  );
}

function CallButton({
  label,
  compact,
  size,
  background,
  onPress,
  children,
}: {
  label: string;
  compact: boolean;
  size: number;
  background: string;
  onPress: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      data-island-hit=""
      onPointerDown={stopGesture}
      onClick={(event) => {
        event.stopPropagation();
        onPress();
      }}
      style={{
        width: compact ? size : undefined,
        height: size,
        minWidth: compact ? size : 96,
        padding: compact ? 0 : "0 14px",
        border: "none",
        borderRadius: size / 2,
        background,
        color: "#fff",
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        gap: 6,
        cursor: "pointer",
        flexShrink: 0,
        fontSize: 13,
        fontWeight: 600,
      }}
    >
      {children}
    </button>
  );
}

function PhoneIcon({ down = false }: { down?: boolean }) {
  return (
    <svg
      width="14"
      height="14"
      viewBox="0 0 24 24"
      fill="none"
      aria-hidden="true"
      style={down ? { transform: "rotate(135deg)" } : undefined}
    >
      <path
        d="M7 3.5h2.2c.4 0 .8.3.9.7l.8 2.4a1 1 0 0 1-.3 1L8.8 9.2a12.5 12.5 0 0 0 6 6l1.6-1.8a1 1 0 0 1 1-.3l2.4.8c.4.1.7.5.7.9V17a2 2 0 0 1-2.2 2A15.5 15.5 0 0 1 5 8.7 2 2 0 0 1 7 3.5Z"
        fill="#fff"
      />
    </svg>
  );
}
