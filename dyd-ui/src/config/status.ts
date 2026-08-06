import type { SystemStatus } from "../stores/system-store";

export type StatusFeature = "volume" | "wifi" | "bluetooth" | "battery" | "brightness";

export interface StatusFeatureConfig {
  enabled: boolean;
  label: string;
}

export const DEFAULT_STATUS_ORDER: StatusFeature[] = ["volume", "wifi", "bluetooth", "battery", "brightness"];

export const STATUS_FEATURES: Record<StatusFeature, StatusFeatureConfig> = {
  volume: { enabled: true, label: "Volume" },
  wifi: { enabled: true, label: "WiFi" },
  bluetooth: { enabled: true, label: "Bluetooth" },
  battery: { enabled: true, label: "Battery" },
  brightness: { enabled: true, label: "Brightness" },
};

/** A feature is "active" only while something is actually happening, e.g.
 * the laptop is charging, a network/device is connecting, or audio is
 * playing. Persistent steady-state values (a discharging battery, an already
 * connected wifi/bluetooth) are considered idle so the island stays clean. */
export function isStatusFeatureActive(feature: StatusFeature, status: SystemStatus): boolean {
  switch (feature) {
    case "battery": {
      const b = status.battery;
      if (!b || !b.is_present) return false;
      const charging = b.state === "charging" || b.state === "pending-charge";
      const critical = b.percentage < 20 && !charging;
      return charging || critical;
    }
    case "wifi":
      return status.wifi?.state === "connecting";
    case "bluetooth": {
      const bt = status.bluetooth;
      if (!bt || bt.present === false) return false;
      return bt.discoverable;
    }
    default:
      return false;
  }
}
