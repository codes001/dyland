import { useRef, useState } from "react";
import { useSystemStore } from "../../stores/system-store";
import { usePillStore } from "../../stores/pill-store";
import { ipcClient } from "../../ipc/client";
import { StatusExpandedFallback } from "./StatusExpandedView";

function deviceIcon(icon: string | null): string {
  if (!icon) return "computer";
  if (icon.includes("audio") || icon.includes("headset") || icon.includes("headphone")) return "audio-headset";
  if (icon.includes("phone")) return "phone";
  if (icon.includes("input")) return "input-keyboard";
  return icon;
}

export default function BluetoothExpandedView() {
  const bluetooth = useSystemStore((s) => s.bluetooth);
  const clearSelected = useSystemStore((s) => s.clearSelectedStatusFeature);
  const [pending, setPending] = useState(false);
  const pendingRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const back = () => {
    clearSelected();
    usePillStore.getState().transitionTo("active");
  };

  if (!bluetooth) return <StatusExpandedFallback feature="bluetooth" />;

  const connected = bluetooth.devices.filter((d) => d.connected);
  const toggle = async () => {
    if (pending) return;
    setPending(true);
    const cmd = JSON.stringify({ type: "SetBluetoothPowered", payload: { powered: !bluetooth.powered } });
    try {
      await ipcClient.sendCommand(cmd);
    } finally {
      if (pendingRef.current) clearTimeout(pendingRef.current);
      pendingRef.current = setTimeout(() => setPending(false), 600);
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12, width: "100%" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <button
          onClick={back}
          style={{ background: "none", border: "none", color: "#ffffff", opacity: 0.6, fontSize: 12, cursor: "pointer" }}
          aria-label="Back to status"
        >
          ←
        </button>
        <span style={{ fontSize: 12, opacity: 0.6 }}>Bluetooth</span>
      </div>

      <button
        onClick={toggle}
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "8px 10px",
          borderRadius: 10,
          border: "1px solid rgba(255,255,255,0.15)",
          background: "rgba(255,255,255,0.05)",
          color: "#ffffff",
          cursor: "pointer",
        }}
        aria-pressed={bluetooth.powered}
        disabled={pending}
      >
        <span style={{ fontSize: 13 }}>{bluetooth.powered ? "Bluetooth" : "Bluetooth Off"}</span>
        <span
          style={{
            width: 34,
            height: 20,
            borderRadius: 999,
            background: bluetooth.powered ? "rgba(74,222,128,0.9)" : "rgba(255,255,255,0.15)",
            position: "relative",
            transition: "background 0.2s",
          }}
        >
          <span
            style={{
              position: "absolute",
              top: 2,
              left: bluetooth.powered ? 16 : 2,
              width: 16,
              height: 16,
              borderRadius: "50%",
              background: "#111111",
              transition: "left 0.2s",
            }}
          />
        </span>
      </button>

      {connected.length === 0 ? (
        <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: 80 }}>
          <span style={{ fontSize: 13, opacity: 0.5 }}>No devices connected</span>
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          {connected.map((d) => (
            <div
              key={d.address}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 8,
                padding: "6px 8px",
                borderRadius: 8,
                background: "rgba(255,255,255,0.04)",
              }}
            >
              <span style={{ opacity: 0.7, fontSize: 13 }} aria-hidden>
                {deviceIcon(d.icon)}
              </span>
              <span style={{ fontSize: 12, fontWeight: 500 }}>{d.name}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
