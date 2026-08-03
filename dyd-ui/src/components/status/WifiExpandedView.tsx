import { useSystemStore } from "../../stores/system-store";
import { usePillStore } from "../../stores/pill-store";
import { StatusExpandedFallback } from "./StatusExpandedView";

function frequencyBand(mhz: number | null): string | null {
  if (mhz === null || mhz === undefined) return null;
  if (mhz < 5000) return "2.4 GHz";
  if (mhz < 6000) return "5 GHz";
  return "6 GHz";
}

export default function WifiExpandedView() {
  const wifi = useSystemStore((s) => s.wifi);
  const clearSelected = useSystemStore((s) => s.clearSelectedStatusFeature);

  const back = () => {
    clearSelected();
    usePillStore.getState().transitionTo("active");
  };

  if (!wifi) return <StatusExpandedFallback feature="wifi" />;

  const connected = wifi.state === "connected";
  const ssid = wifi.ssid ?? "Hidden Network";
  const band = frequencyBand(wifi.frequency_mhz);

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
        <span style={{ fontSize: 12, opacity: 0.6 }}>WiFi</span>
      </div>

      {!connected ? (
        <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: 80 }}>
          <span style={{ fontSize: 14, opacity: 0.5 }}>No WiFi Connection</span>
        </div>
      ) : (
        <>
          <div style={{ textAlign: "center" }}>
            <div style={{ fontSize: 20, fontWeight: 600 }}>{ssid}</div>
            <div style={{ fontSize: 12, opacity: 0.6, marginTop: 4 }}>
              {[band, wifi.security].filter(Boolean).join(" · ")}
            </div>
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11, opacity: 0.6 }}>
              <span>Signal Strength</span>
              <span>{wifi.strength}%</span>
            </div>
            <div style={{ width: "100%", height: 6, background: "rgba(255,255,255,0.12)", borderRadius: 999, overflow: "hidden" }}>
              <div
                style={{
                  width: `${wifi.strength}%`,
                  height: "100%",
                  background: wifi.strength < 30 ? "#f97316" : "#4ade80",
                  borderRadius: 999,
                }}
              />
            </div>
          </div>

          {wifi.ip_address && (
            <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12 }}>
              <span style={{ opacity: 0.6 }}>IP Address</span>
              <span>{wifi.ip_address}</span>
            </div>
          )}
        </>
      )}
    </div>
  );
}
