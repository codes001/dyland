import { useConfigStore } from "../../stores/config-store";

export default function AccessibilitySettings() {
  const accessibility = useConfigStore((s) => s.config.accessibility);
  const updateConfig = useConfigStore((s) => s.updateConfig);

  const fontScale = accessibility?.font_scale ?? 1.0;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
      <h3 style={{ fontSize: 14, margin: 0, color: "var(--pill-text)" }}>Accessibility</h3>

      <label style={{ display: "flex", alignItems: "center", justifyContent: "space-between", cursor: "pointer" }}>
        <span style={{ fontSize: 13, color: "var(--pill-text)" }}>Reduce Motion</span>
        <input
          type="checkbox"
          checked={accessibility?.reduce_motion ?? false}
          onChange={(e) => updateConfig({ accessibility: { ...accessibility, reduce_motion: e.target.checked } })}
          style={{ width: 16, height: 16, cursor: "pointer" }}
        />
      </label>

      <label style={{ display: "flex", alignItems: "center", justifyContent: "space-between", cursor: "pointer" }}>
        <span style={{ fontSize: 13, color: "var(--pill-text)" }}>High Contrast Mode</span>
        <input
          type="checkbox"
          checked={accessibility?.high_contrast ?? false}
          onChange={(e) => updateConfig({ accessibility: { ...accessibility, high_contrast: e.target.checked } })}
          style={{ width: 16, height: 16, cursor: "pointer" }}
        />
      </label>

      <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <span style={{ fontSize: 13, color: "var(--pill-text)" }}>Font Size Scale</span>
          <span style={{ fontSize: 12, fontFamily: "monospace" }}>{Math.round(fontScale * 100)}%</span>
        </div>
        <input
          type="range"
          min="0.75"
          max="1.50"
          step="0.05"
          value={fontScale}
          onChange={(e) =>
            updateConfig({
              accessibility: {
                ...accessibility,
                font_scale: parseFloat(e.target.value),
              },
            })
          }
          style={{ width: "100%", cursor: "pointer" }}
        />
      </div>
    </div>
  );
}
