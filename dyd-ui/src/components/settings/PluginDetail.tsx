import { useState } from "react";
import { usePluginStore } from "../../stores/plugin-store";
import { ipcClient } from "../../ipc/client";
import { badgeColor, statusLabel, BADGE_HEX, type PluginInfo } from "../../types/plugin";
import PluginConfigForm from "./PluginConfigForm";

const btn: React.CSSProperties = {
  padding: "6px 12px",
  borderRadius: 6,
  border: "1px solid rgba(255,255,255,0.2)",
  background: "rgba(255,255,255,0.05)",
  color: "#fff",
  fontSize: 12,
  cursor: "pointer",
};

const dangerBtn: React.CSSProperties = {
  ...btn,
  border: "1px solid rgba(248,81,73,0.4)",
  color: "#f85149",
};

/** Detail panel for the selected plugin: status, enable/disable, error-state
 * actions, uninstall (with confirm + core warning), and the config form. */
export default function PluginDetail() {
  const plugin = usePluginStore((s) => {
    const name = s.selectedName;
    return name ? (s.plugins.find((p) => p.name === name) ?? null) : null;
  });
  const setSelected = usePluginStore((s) => s.setSelected);
  const removePlugin = usePluginStore((s) => s.removePlugin);
  const [confirmingUninstall, setConfirmingUninstall] = useState(false);

  if (!plugin) return null;

  const color = BADGE_HEX[badgeColor(plugin)];
  const isError = plugin.state === "error";
  const enabled = !plugin.disabled;

  const toggleEnabled = () => {
    void ipcClient.setPluginEnabled(plugin.name, !enabled);
  };

  // "Reset" on an errored plugin re-enables it (clears the disabled flag so the
  // daemon can reload it from its Error state).
  const resetPlugin = () => {
    void ipcClient.setPluginEnabled(plugin.name, true);
  };

  const confirmUninstall = () => {
    void ipcClient.uninstallPlugin(plugin.name);
    removePlugin(plugin.name);
    setConfirmingUninstall(false);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
      <Header plugin={plugin} color={color} onBack={() => setSelected(null)} />

      {plugin.last_error && (
        <div
          style={{
            fontSize: 11,
            color: "#f85149",
            background: "rgba(248,81,73,0.08)",
            border: "1px solid rgba(248,81,73,0.25)",
            borderRadius: 6,
            padding: 8,
          }}
        >
          {plugin.last_error}
          {plugin.error_count > 0 && ` (×${plugin.error_count})`}
        </div>
      )}

      <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
        <button onClick={toggleEnabled} style={btn}>
          {enabled ? "Disable" : "Enable"}
        </button>
        {isError && (
          <button onClick={resetPlugin} style={btn}>
            Reset
          </button>
        )}
        {!confirmingUninstall ? (
          <button onClick={() => setConfirmingUninstall(true)} style={dangerBtn}>
            Uninstall
          </button>
        ) : (
          <ConfirmUninstall
            plugin={plugin}
            onConfirm={confirmUninstall}
            onCancel={() => setConfirmingUninstall(false)}
          />
        )}
      </div>

      <section style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        <h3 style={{ fontSize: 13, margin: 0, opacity: 0.8 }}>Configuration</h3>
        <PluginConfigForm
          pluginName={plugin.name}
          schema={plugin.config_schema}
          config={plugin.config}
        />
      </section>
    </div>
  );
}

function Header({
  plugin,
  color,
  onBack,
}: {
  plugin: PluginInfo;
  color: string;
  onBack: () => void;
}) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
      <button
        onClick={onBack}
        aria-label="Back to plugin list"
        style={{ ...btn, padding: "2px 8px" }}
      >
        ‹
      </button>
      <span
        style={{ width: 8, height: 8, borderRadius: "50%", background: color, flexShrink: 0 }}
      />
      <div style={{ flex: 1 }}>
        <div style={{ fontSize: 14, fontWeight: 700 }}>{plugin.name}</div>
        <div style={{ fontSize: 11, opacity: 0.5 }}>
          v{plugin.version} · {statusLabel(plugin)}
          {plugin.is_core && " · system plugin"}
        </div>
      </div>
    </div>
  );
}

function ConfirmUninstall({
  plugin,
  onConfirm,
  onCancel,
}: {
  plugin: PluginInfo;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: 8,
        padding: 10,
        borderRadius: 6,
        border: "1px solid rgba(248,81,73,0.3)",
        background: "rgba(248,81,73,0.06)",
        width: "100%",
      }}
    >
      <span style={{ fontSize: 12 }}>
        Uninstall <strong>{plugin.name}</strong>?
      </span>
      {plugin.is_core && (
        <span style={{ fontSize: 11, color: "#d29922" }}>
          Warning: this is a system plugin. Removing it may break core functionality.
        </span>
      )}
      <div style={{ display: "flex", gap: 8 }}>
        <button onClick={onConfirm} style={dangerBtn}>
          Confirm uninstall
        </button>
        <button onClick={onCancel} style={btn}>
          Cancel
        </button>
      </div>
    </div>
  );
}
