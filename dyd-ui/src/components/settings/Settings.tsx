import { useEffect } from "react";
import { usePluginStore } from "../../stores/plugin-store";
import { ipcClient } from "../../ipc/client";
import PluginList from "./PluginList";
import PluginDetail from "./PluginDetail";

interface Props {
  onClose: () => void;
}

/**
 * Minimal Settings container hosting the plugin-management views. The app has no
 * full settings surface yet (Epic 7); this is the smallest shell needed for
 * story 6-5. Requests the plugin list on mount; detail view replaces the list
 * when a plugin is selected.
 */
export default function Settings({ onClose }: Props) {
  const selectedName = usePluginStore((s) => s.selectedName);

  useEffect(() => {
    void ipcClient.getPlugins();
  }, []);

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: 12,
        padding: 16,
        minWidth: 320,
        maxWidth: 420,
        maxHeight: 480,
        overflowY: "auto",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <h2 style={{ fontSize: 15, margin: 0 }}>Plugins</h2>
        <button
          onClick={onClose}
          aria-label="Close settings"
          style={{
            padding: "4px 10px",
            borderRadius: 6,
            border: "1px solid rgba(255,255,255,0.2)",
            background: "transparent",
            color: "#fff",
            fontSize: 12,
            cursor: "pointer",
          }}
        >
          Close
        </button>
      </div>

      {selectedName ? <PluginDetail /> : <PluginList />}
    </div>
  );
}
