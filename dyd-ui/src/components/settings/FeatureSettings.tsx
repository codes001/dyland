import { useConfigStore } from "../../stores/config-store";
import type { FeatureToggles } from "../../../../types/src/index";

const FEATURE_LABELS: Record<keyof FeatureToggles, string> = {
  battery: "Battery Monitoring",
  wifi: "Wi-Fi Status",
  bluetooth: "Bluetooth Status",
  volume: "Volume Control",
  brightness: "Brightness Control",
  media: "Media Player Integration",
  notifications: "Notification Interception",
};

export default function FeatureSettings() {
  const features = useConfigStore((s) => s.config.features);
  const updateConfig = useConfigStore((s) => s.updateConfig);

  const toggleFeature = (key: keyof FeatureToggles) => {
    void updateConfig({
      features: {
        ...features,
        [key]: !features[key],
      },
    });
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <h3 style={{ fontSize: 14, margin: 0, color: "var(--pill-text)" }}>Feature Toggles</h3>
      <p style={{ fontSize: 12, opacity: 0.7, margin: 0 }}>
        Enable or disable individual dynamic island features and status indicators.
      </p>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        {(Object.keys(FEATURE_LABELS) as (keyof FeatureToggles)[]).map((key) => (
          <label
            key={key}
            style={{ display: "flex", alignItems: "center", justifyContent: "space-between", cursor: "pointer", padding: "4px 0" }}
          >
            <span style={{ fontSize: 13, color: "var(--pill-text)" }}>{FEATURE_LABELS[key]}</span>
            <input
              type="checkbox"
              checked={features[key]}
              onChange={() => toggleFeature(key)}
              style={{ width: 16, height: 16, cursor: "pointer" }}
            />
          </label>
        ))}
      </div>
    </div>
  );
}
