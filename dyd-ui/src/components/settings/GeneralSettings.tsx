import { useConfigStore } from "../../stores/config-store";

export default function GeneralSettings() {
  const general = useConfigStore((s) => s.config.general);
  const updateConfig = useConfigStore((s) => s.updateConfig);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <h3 style={{ fontSize: 14, margin: 0, color: "var(--pill-text)" }}>General Settings</h3>
      <label style={{ display: "flex", alignItems: "center", justifyContent: "space-between", cursor: "pointer" }}>
        <span style={{ fontSize: 13, color: "var(--pill-text)" }}>Start on Login</span>
        <input
          type="checkbox"
          checked={general.start_on_login}
          onChange={(e) => updateConfig({ general: { ...general, start_on_login: e.target.checked } })}
          style={{ width: 16, height: 16, cursor: "pointer" }}
        />
      </label>
    </div>
  );
}
