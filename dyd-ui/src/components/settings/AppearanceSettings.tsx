import { useConfigStore } from "../../stores/config-store";
import type { ThemePreference } from "../../../../types/src/index";

export default function AppearanceSettings() {
  const appearance = useConfigStore((s) => s.config.appearance);
  const updateConfig = useConfigStore((s) => s.updateConfig);

  const setTheme = (theme: ThemePreference) => {
    void updateConfig({ appearance: { ...appearance, theme } });
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <h3 style={{ fontSize: 14, margin: 0, color: "var(--pill-text)" }}>Theme & Appearance</h3>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        <span style={{ fontSize: 13, color: "var(--pill-text)" }}>Theme Mode</span>
        <div style={{ display: "flex", gap: 8 }}>
          {(["dark", "light", "system"] as const).map((t) => (
            <button
              key={t}
              onClick={() => setTheme(t)}
              style={{
                flex: 1,
                padding: "6px 10px",
                borderRadius: 6,
                border: appearance.theme === t ? "1px solid #3b82f6" : "1px solid var(--pill-border)",
                background: appearance.theme === t ? "rgba(59, 130, 246, 0.2)" : "rgba(255, 255, 255, 0.05)",
                color: "var(--pill-text)",
                fontSize: 12,
                textTransform: "capitalize",
                cursor: "pointer",
              }}
            >
              {t}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
