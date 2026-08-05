import { usePluginStore } from "../../stores/plugin-store";
import { badgeColor, statusLabel, BADGE_HEX, type PluginInfo } from "../../types/plugin";

/**
 * List of installed plugins with a name, version, and status badge. Clicking a
 * row selects it (opening the detail view). Shows an empty state and a count
 * summary (AC1, AC2, AC4).
 */
export default function PluginList() {
  const plugins = usePluginStore((s) => s.plugins);
  const selectedName = usePluginStore((s) => s.selectedName);
  const setSelected = usePluginStore((s) => s.setSelected);

  if (plugins.length === 0) {
    return (
      <div style={{ padding: 16, textAlign: "center", opacity: 0.6 }}>
        <p style={{ fontSize: 13, margin: 0 }}>No plugins installed.</p>
        <p style={{ fontSize: 12, margin: "4px 0 0" }}>Browse the plugin marketplace.</p>
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      <div style={{ fontSize: 11, opacity: 0.5, padding: "0 4px" }}>
        {plugins.length} plugin{plugins.length === 1 ? "" : "s"} installed
      </div>
      {plugins.map((p) => (
        <PluginRow
          key={p.name}
          plugin={p}
          selected={p.name === selectedName}
          onClick={() => setSelected(p.name)}
        />
      ))}
    </div>
  );
}

function PluginRow({
  plugin,
  selected,
  onClick,
}: {
  plugin: PluginInfo;
  selected: boolean;
  onClick: () => void;
}) {
  const color = BADGE_HEX[badgeColor(plugin)];
  return (
    <button
      onClick={onClick}
      style={{
        display: "flex",
        alignItems: "center",
        gap: 10,
        padding: "8px 10px",
        borderRadius: 8,
        border: "1px solid rgba(255,255,255,0.08)",
        background: selected ? "rgba(255,255,255,0.08)" : "rgba(255,255,255,0.03)",
        color: "#fff",
        cursor: "pointer",
        textAlign: "left",
        width: "100%",
      }}
    >
      <span
        aria-label={statusLabel(plugin)}
        title={statusLabel(plugin)}
        style={{
          width: 8,
          height: 8,
          borderRadius: "50%",
          background: color,
          flexShrink: 0,
        }}
      />
      <span style={{ flex: 1, fontSize: 13, fontWeight: 600 }}>{plugin.name}</span>
      <span style={{ fontSize: 11, opacity: 0.5 }}>v{plugin.version}</span>
    </button>
  );
}
