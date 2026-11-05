import { useSystemStore } from "../../stores/system-store";
import { useTransientStore } from "../../stores/transient-store";
import { usePillStore } from "../../stores/pill-store";
import {
  DEFAULT_STATUS_ORDER,
  STATUS_FEATURES,
  isStatusFeatureActive,
  type StatusFeature,
} from "../../config/status";
import BatteryIndicator from "./BatteryIndicator";
import WifiIndicator from "./WifiIndicator";
import BluetoothIndicator from "./BluetoothIndicator";
import VolumeIndicator from "./VolumeIndicator";
import BrightnessIndicator from "./BrightnessIndicator";
import IdleClock from "../IdleClock";

export default function StatusIconRow() {
  const battery = useSystemStore((s) => s.battery);
  const wifi = useSystemStore((s) => s.wifi);
  const bluetooth = useSystemStore((s) => s.bluetooth);
  const volume = useSystemStore((s) => s.volume);
  const brightness = useSystemStore((s) => s.brightness);
  const transient = useTransientStore((s) => s.active);
  const selectFeature = useSystemStore((s) => s.selectStatusFeature);

  const system = { battery, wifi, bluetooth, volume, brightness };

  const activeFeatures = DEFAULT_STATUS_ORDER.filter((f) => {
    if (!STATUS_FEATURES[f].enabled) return false;
    if (f === "volume" || f === "brightness") return transient[f];
    return isStatusFeatureActive(f, system);
  });

  if (activeFeatures.length === 0) {
    return <IdleClock />;
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
      {activeFeatures.map((f) => {
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
