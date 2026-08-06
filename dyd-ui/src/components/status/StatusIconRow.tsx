import { useSystemStore } from "../../stores/system-store";
import { usePillStore } from "../../stores/pill-store";
import { DEFAULT_STATUS_ORDER, STATUS_FEATURES, type StatusFeature } from "../../config/status";
import BatteryIndicator from "./BatteryIndicator";
import WifiIndicator from "./WifiIndicator";
import BluetoothIndicator from "./BluetoothIndicator";
import VolumeIndicator from "./VolumeIndicator";
import BrightnessIndicator from "./BrightnessIndicator";

export default function StatusIconRow() {
  const battery = useSystemStore((s) => s.battery);
  const wifi = useSystemStore((s) => s.wifi);
  const bluetooth = useSystemStore((s) => s.bluetooth);
  const volume = useSystemStore((s) => s.volume);
  const brightness = useSystemStore((s) => s.brightness);
  const selectFeature = useSystemStore((s) => s.selectStatusFeature);

  const availability: Record<StatusFeature, boolean> = {
    battery: battery?.is_present === true,
    wifi: wifi != null && wifi.state !== "none",
    bluetooth: bluetooth !== null,
    volume: volume !== null,
    brightness: brightness?.has_backlight === true,
  };

  const visibleIcons = DEFAULT_STATUS_ORDER.filter(
    (f) => STATUS_FEATURES[f].enabled && availability[f],
  );

  if (visibleIcons.length === 0) {
    return (
      <span style={{ color: "var(--pill-text)", fontSize: 12, opacity: 0.5, fontWeight: 500, letterSpacing: "0.05em" }}>
        dyland
      </span>
    );
  }

  const openFeature = (feature: StatusFeature) => {
    selectFeature(feature);
    const current = usePillStore.getState();
    if (current.state === "dormant") {
      current.transitionTo("active");
      usePillStore.getState().transitionTo("expanded");
    } else {
      current.transitionTo("expanded");
    }
  };

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        gap: 10,
        maxWidth: 280,
        flexWrap: "nowrap",
        overflow: "hidden",
      }}
    >
      {visibleIcons.map((f) => {
        const common = {
          size: 16,
          showPercentage: true,
          showSsid: true,
          showCount: true,
          onClick: () => openFeature(f),
        };
        switch (f) {
          case "battery":
            return <BatteryIndicator key={f} {...common} />;
          case "wifi":
            return <WifiIndicator key={f} {...common} />;
          case "bluetooth":
            return <BluetoothIndicator key={f} {...common} />;
          case "volume":
            return <VolumeIndicator key={f} {...common} />;
          case "brightness":
            return <BrightnessIndicator key={f} {...common} />;
        }
      })}
    </div>
  );
}
