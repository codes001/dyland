import { useEffect, useState } from "react";
import { useConfigStore } from "../../stores/config-store";
import { usePluginStore } from "../../stores/plugin-store";
import { ipcClient } from "../../ipc/client";
import GeneralSettings from "./GeneralSettings";
import AppearanceSettings from "./AppearanceSettings";
import FeatureSettings from "./FeatureSettings";
import NotificationSettings from "./NotificationSettings";
import ShortcutSettings from "./ShortcutSettings";
import AccessibilitySettings from "./AccessibilitySettings";
import PluginList from "./PluginList";
import PluginDetail from "./PluginDetail";
import AboutSettings from "./AboutSettings";

interface Props {
  onClose: () => void;
}

type CategoryKey =
  | "general"
  | "theme"
  | "features"
  | "notifications"
  | "shortcuts"
  | "accessibility"
  | "plugins"
  | "about";

const CATEGORIES: { key: CategoryKey; label: string }[] = [
  { key: "general", label: "General" },
  { key: "theme", label: "Theme" },
  { key: "features", label: "Features" },
  { key: "notifications", label: "Notifications" },
  { key: "shortcuts", label: "Shortcuts" },
  { key: "accessibility", label: "Accessibility" },
  { key: "plugins", label: "Plugins" },
  { key: "about", label: "About" },
];

export default function Settings({ onClose }: Props) {
  const lastCategory = useConfigStore((s) => s.config.last_category);
  const updateConfig = useConfigStore((s) => s.updateConfig);
  const resetToDefaults = useConfigStore((s) => s.resetToDefaults);
  const selectedPlugin = usePluginStore((s) => s.selectedName);

  const [activeCategory, setActiveCategory] = useState<CategoryKey>(
    (lastCategory as CategoryKey) || "general"
  );

  useEffect(() => {
    void ipcClient.getPlugins();
    void ipcClient.getConfig();
  }, []);

  const selectCategory = (cat: CategoryKey) => {
    setActiveCategory(cat);
    void updateConfig({ last_category: cat });
  };

  const renderContent = () => {
    switch (activeCategory) {
      case "general":
        return <GeneralSettings />;
      case "theme":
        return <AppearanceSettings />;
      case "features":
        return <FeatureSettings />;
      case "notifications":
        return <NotificationSettings />;
      case "shortcuts":
        return <ShortcutSettings />;
      case "accessibility":
        return <AccessibilitySettings />;
      case "plugins":
        return selectedPlugin ? <PluginDetail /> : <PluginList />;
      case "about":
        return <AboutSettings />;
    }
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Settings Panel"
      style={{
        display: "flex",
        flexDirection: "column",
        gap: 12,
        padding: 16,
        minWidth: 360,
        maxWidth: 480,
        maxHeight: 520,
        color: "var(--pill-text)",
      }}
    >
      {/* Header */}
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <h2 style={{ fontSize: 16, margin: 0, fontWeight: 600 }}>Settings</h2>
        <div style={{ display: "flex", gap: 8 }}>
          <button
            onClick={() => void resetToDefaults()}
            aria-label="Reset all settings to defaults"
            style={{
              padding: "4px 8px",
              borderRadius: 6,
              border: "1px solid var(--pill-border)",
              background: "transparent",
              color: "var(--pill-text)",
              fontSize: 11,
              opacity: 0.8,
              cursor: "pointer",
            }}
          >
            Reset Defaults
          </button>
          <button
            onClick={onClose}
            aria-label="Close settings"
            style={{
              padding: "4px 10px",
              borderRadius: 6,
              border: "1px solid var(--pill-border)",
              background: "transparent",
              color: "var(--pill-text)",
              fontSize: 12,
              cursor: "pointer",
            }}
          >
            Close
          </button>
        </div>
      </div>

      {/* Main tabbed view */}
      <div style={{ display: "flex", gap: 12, flex: 1, minHeight: 320, overflow: "hidden" }}>
        {/* Category Tabs / Sidebar */}
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: 4,
            width: 110,
            borderRight: "1px solid var(--pill-border)",
            paddingRight: 8,
          }}
        >
          {CATEGORIES.map((cat) => (
            <button
              key={cat.key}
              onClick={() => selectCategory(cat.key)}
              style={{
                textAlign: "left",
                padding: "6px 8px",
                borderRadius: 6,
                border: "none",
                background: activeCategory === cat.key ? "rgba(255, 255, 255, 0.12)" : "transparent",
                color: "var(--pill-text)",
                fontSize: 12,
                fontWeight: activeCategory === cat.key ? 600 : 400,
                cursor: "pointer",
              }}
            >
              {cat.label}
            </button>
          ))}
        </div>

        {/* Active Category Content */}
        <div style={{ flex: 1, overflowY: "auto", paddingRight: 4 }}>
          {renderContent()}
        </div>
      </div>
    </div>
  );
}
