import { useState, useEffect } from "react";
import { connectionStore, type ConnectionStatus } from "../stores/connection";

const indicatorColors: Record<ConnectionStatus, string> = {
  connected: "#30D158",
  connecting: "#FF9F0A",
  reconnecting: "#FF9F0A",
  disconnected: "#FF453A",
};

const statusLabels: Record<ConnectionStatus, string> = {
  connected: "",
  connecting: "",
  reconnecting: "",
  disconnected: "Disconnected",
};

export function ConnectionIndicator() {
  const [status, setStatus] = useState<ConnectionStatus>("disconnected");

  useEffect(() => {
    const unsub = connectionStore.subscribe(setStatus);
    return () => unsub();
  }, []);

  if (status === "connected" || status === "connecting" || status === "reconnecting") return null;

  return (
    <div
      style={{
        position: "fixed",
        top: 10,
        right: 16,
        display: "flex",
        alignItems: "center",
        gap: 4,
        fontSize: 10,
        color: indicatorColors[status],
        opacity: 0.8,
        pointerEvents: "none",
        zIndex: 10000,
      }}
    >
      <span
        style={{
          width: 6,
          height: 6,
          borderRadius: "50%",
          backgroundColor: indicatorColors[status],
        }}
      />
      <span>{statusLabels[status]}</span>
    </div>
  );
}
