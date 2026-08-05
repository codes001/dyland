import { useConfigStore } from "../../stores/config-store";
import ShortcutRecorder from "./ShortcutRecorder";

const ACTION_DESCRIPTIONS: Record<string, string> = {
  expand: "Expand Current Activity",
  collapse: "Collapse Expanded View",
  prev_activity: "Previous Activity in Queue",
  next_activity: "Next Activity in Queue",
  toggle_dnd: "Toggle Do Not Disturb Mode",
  open_settings: "Open / Close Settings Panel",
};

export default function ShortcutSettings() {
  const shortcuts = useConfigStore((s) => s.config.shortcuts);
  const updateConfig = useConfigStore((s) => s.updateConfig);

  const updateBinding = (actionKey: string, keyCombo: string) => {
    void updateConfig({
      shortcuts: {
        bindings: {
          ...shortcuts.bindings,
          [actionKey]: keyCombo,
        },
      },
    });
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <h3 style={{ fontSize: 14, margin: 0, color: "var(--pill-text)" }}>Keyboard Shortcuts</h3>
      <p style={{ fontSize: 12, opacity: 0.7, margin: 0 }}>
        Customize hotkeys for operating the Dynamic Island. Click a shortcut button to rebind.
      </p>

      <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        {Object.entries(ACTION_DESCRIPTIONS).map(([actionKey, label]) => (
          <ShortcutRecorder
            key={actionKey}
            actionLabel={label}
            value={shortcuts?.bindings?.[actionKey] || ""}
            onChange={(combo) => updateBinding(actionKey, combo)}
          />
        ))}
      </div>
    </div>
  );
}
