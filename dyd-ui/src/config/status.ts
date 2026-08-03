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
