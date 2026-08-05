import { useState } from "react";
import { useConfigStore } from "../../stores/config-store";
import type { AppNotificationRule } from "../../../../types/src/index";

export default function NotificationSettings() {
  const notifications = useConfigStore((s) => s.config.notifications);
  const updateConfig = useConfigStore((s) => s.updateConfig);
  const [newAppName, setNewAppName] = useState("");

  const toggleDnd = () => {
    void updateConfig({
      notifications: {
        ...notifications,
        dnd_enabled: !notifications?.dnd_enabled,
      },
    });
  };

  const updateAppRule = (appName: string, patch: Partial<AppNotificationRule>) => {
    const existing = notifications?.app_rules?.[appName] || { allow: true, show_silently: false, queue_only: false };
    void updateConfig({
      notifications: {
        ...notifications,
        app_rules: {
          ...(notifications?.app_rules || {}),
          [appName]: { ...existing, ...patch },
        },
      },
    });
  };

  const removeAppRule = (appName: string) => {
    const nextRules = { ...(notifications?.app_rules || {}) };
    delete nextRules[appName];
    void updateConfig({
      notifications: {
        ...notifications,
        app_rules: nextRules,
      },
    });
  };

  const addAppRule = () => {
    if (!newAppName.trim()) return;
    updateAppRule(newAppName.trim(), { allow: true, show_silently: false, queue_only: false });
    setNewAppName("");
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      <h3 style={{ fontSize: 14, margin: 0, color: "var(--pill-text)" }}>Notification Settings</h3>

      <label style={{ display: "flex", alignItems: "center", justifyContent: "space-between", cursor: "pointer" }}>
        <span style={{ fontSize: 13, color: "var(--pill-text)" }}>Do Not Disturb (DND) Mode</span>
        <input
          type="checkbox"
          checked={notifications.dnd_enabled}
          onChange={toggleDnd}
          style={{ width: 16, height: 16, cursor: "pointer" }}
        />
      </label>

      <hr style={{ borderColor: "var(--pill-border)", margin: "4px 0" }} />

      <h4 style={{ fontSize: 13, margin: 0, color: "var(--pill-text)" }}>Per-App Notification Rules</h4>

      <div style={{ display: "flex", gap: 6 }}>
        <input
          type="text"
          placeholder="App name (e.g. Slack, Discord)..."
          value={newAppName}
          onChange={(e) => setNewAppName(e.target.value)}
          style={{
            flex: 1,
            padding: "4px 8px",
            borderRadius: 6,
            border: "1px solid var(--pill-border)",
            background: "rgba(255,255,255,0.05)",
            color: "var(--pill-text)",
            fontSize: 12,
          }}
        />
        <button
          onClick={addAppRule}
          style={{
            padding: "4px 10px",
            borderRadius: 6,
            border: "1px solid var(--pill-border)",
            background: "#3b82f6",
            color: "#fff",
            fontSize: 12,
            cursor: "pointer",
          }}
        >
          Add
        </button>
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: 8, maxHeight: 200, overflowY: "auto" }}>
        {Object.entries(notifications.app_rules || {}).map(([appName, rule]) => (
          <div
            key={appName}
            style={{
              padding: 8,
              borderRadius: 6,
              border: "1px solid var(--pill-border)",
              background: "rgba(255,255,255,0.03)",
              display: "flex",
              flexDirection: "column",
              gap: 4,
            }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <strong style={{ fontSize: 12 }}>{appName}</strong>
              <button
                onClick={() => removeAppRule(appName)}
                style={{ background: "transparent", border: "none", color: "#ef4444", fontSize: 11, cursor: "pointer" }}
              >
                Remove
              </button>
            </div>
            <div style={{ display: "flex", gap: 12, fontSize: 11 }}>
              <label style={{ display: "flex", alignItems: "center", gap: 4 }}>
                <input
                  type="checkbox"
                  checked={rule.allow}
                  onChange={(e) => updateAppRule(appName, { allow: e.target.checked })}
                />
                Allow
              </label>
              <label style={{ display: "flex", alignItems: "center", gap: 4 }}>
                <input
                  type="checkbox"
                  checked={rule.show_silently}
                  onChange={(e) => updateAppRule(appName, { show_silently: e.target.checked })}
                />
                Silent
              </label>
              <label style={{ display: "flex", alignItems: "center", gap: 4 }}>
                <input
                  type="checkbox"
                  checked={rule.queue_only}
                  onChange={(e) => updateAppRule(appName, { queue_only: e.target.checked })}
                />
                Queue Only
              </label>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
