import { useSystemStore } from "../../stores/system-store";
import BatteryExpandedView from "./BatteryExpandedView";
import WifiExpandedView from "./WifiExpandedView";
import BluetoothExpandedView from "./BluetoothExpandedView";
import VolumeExpandedView from "./VolumeExpandedView";
import BrightnessExpandedView from "./BrightnessExpandedView";

const LABELS: Record<string, string> = {
  battery: "Battery",
  wifi: "WiFi",
  bluetooth: "Bluetooth",
  volume: "Volume",
  brightness: "Brightness",
};

export function StatusExpandedFallback({ feature }: { feature: string }) {
  return (
    <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: 120 }}>
      <span style={{ fontSize: 13, opacity: 0.5 }}>{LABELS[feature] ?? feature} unavailable</span>
    </div>
  );
}

export default function StatusExpandedView() {
  const feature = useSystemStore((s) => s.selectedStatusFeature);

  switch (feature) {
    case "battery":
      return <BatteryExpandedView />;
    case "wifi":
      return <WifiExpandedView />;
    case "bluetooth":
      return <BluetoothExpandedView />;
    case "volume":
      return <VolumeExpandedView />;
    case "brightness":
      return <BrightnessExpandedView />;
    default:
      return null;
  }
}
