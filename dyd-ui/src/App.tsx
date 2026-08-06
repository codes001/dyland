import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "motion/react";
import { listen } from "@tauri-apps/api/event";
import Pill from "./components/Pill";
import ExpandedView from "./components/ExpandedView";
import ContextMenu, { type ContextMenuItem } from "./components/ContextMenu";
import { ConnectionIndicator } from "./components/ConnectionIndicator";
import StatusIconRow from "./components/status/StatusIconRow";
import StatusExpandedView from "./components/status/StatusExpandedView";
import PillNotificationContent from "./components/notification/PillNotificationContent";
import NotificationExpandedView from "./components/notification/NotificationExpandedView";
import Settings from "./components/settings/Settings";
import ThemeProvider from "./components/theme/ThemeProvider";
import { connectionStore } from "./stores/connection";
import { usePillStore } from "./stores/pill-store";
import { useActivityStore } from "./stores/activity-store";
import { useMediaStore } from "./stores/media-store";
import { useNotificationStore } from "./stores/notification-store";
import { useSystemStore } from "./stores/system-store";
import { useConfigStore } from "./stores/config-store";
import { ipcClient } from "./ipc/client";

function App() {
  const state = usePillStore((s) => s.state);
  const isFullscreen = usePillStore((s) => s.isFullscreen);
  const transitionTo = usePillStore((s) => s.transitionTo);
  const currentActivity = useActivityStore((s) => s.currentActivity);
  const media = useMediaStore((s) => s.media);
  const notification = useNotificationStore((s) => s.notification);
  const selectedStatusFeature = useSystemStore((s) => s.selectedStatusFeature);
  const bindings = useConfigStore((s) => s.config.shortcuts.bindings);
  const dndEnabled = useConfigStore((s) => s.config.notifications.dnd_enabled);
  const updateConfig = useConfigStore((s) => s.updateConfig);

  const [showSettings, setShowSettings] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);

  useEffect(() => {
    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      document.body.dataset.tauri = "true";
    }
    connectionStore.init();
    return () => connectionStore.cleanup();
  }, []);

  useEffect(() => {
    const unlisten = listen<{ status: string }>("daemon-status", (event) => {
      if (event.payload.status === "connected") {
        usePillStore.getState().transitionTo("dormant");
      }
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  useEffect(() => {
    const unsub = usePillStore.subscribe((s, prev) => {
      if (s.state === "active" && prev.state === "expanded") {
        useSystemStore.getState().clearSelectedStatusFeature();
      }
    });
    return unsub;
  }, []);

  // Keyboard shortcut handler
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't trigger hotkeys if typing inside an input/textarea
      const target = e.target as HTMLElement | null;
      if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable)) {
        return;
      }

      const matchKey = (action: string) => {
        const binding = bindings[action];
        if (!binding) return false;
        const key = e.key === " " ? "Space" : e.key;
        return key.toLowerCase() === binding.toLowerCase();
      };

      if (matchKey("expand")) {
        if (state === "active" || state === "dormant") {
          transitionTo("expanded");
        }
      } else if (matchKey("collapse")) {
        if (state === "expanded") {
          transitionTo("active");
          setShowSettings(false);
        }
      } else if (matchKey("open_settings")) {
        setShowSettings((prev) => !prev);
        if (state !== "expanded") transitionTo("expanded");
      } else if (matchKey("toggle_dnd")) {
        void updateConfig({ notifications: { dnd_enabled: !dndEnabled } });
      } else if (matchKey("prev_activity")) {
        void ipcClient.sendCommand(JSON.stringify({ type: "CyclePrev" }));
      } else if (matchKey("next_activity")) {
        void ipcClient.sendCommand(JSON.stringify({ type: "CycleNext" }));
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [bindings, state, dndEnabled, transitionTo, updateConfig]);

  const hasContent = currentActivity !== null || media !== null || notification !== null;

  const contextMenuItems: ContextMenuItem[] = [
    {
      label: "Settings",
      action: () => {
        setShowSettings(true);
        if (state !== "expanded") transitionTo("expanded");
      },
    },
    {
      label: dndEnabled ? "Disable Do Not Disturb" : "Enable Do Not Disturb",
      action: () => {
        void updateConfig({ notifications: { dnd_enabled: !dndEnabled } });
      },
    },
  ];

  return (
    <ThemeProvider>
      <div
        className="flex h-screen w-screen items-start justify-center"
        onClick={() => {
          if (state === "expanded") transitionTo("active");
        }}
      >
        <Pill onLongPress={() => setMenuOpen(true)}>
          {(state === "dormant" || (state === "active" && notification)) && !isFullscreen && (
            <AnimatePresence mode="popLayout">
              {!hasContent ? (
                <motion.div
                  key="status"
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                  transition={{ duration: 0.1, ease: "easeOut" }}
                >
                  <StatusIconRow />
                </motion.div>
              ) : (
                <motion.div
                  key="content"
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                  transition={{ duration: 0.15, ease: "easeIn" }}
                >
                  {notification ? (
                    <PillNotificationContent />
                  ) : (
                    <span style={{ color: "var(--pill-text)", fontSize: 12, opacity: 0.4 }}>dyd</span>
                  )}
                </motion.div>
              )}
            </AnimatePresence>
          )}
        </Pill>

        <ExpandedView onClose={() => transitionTo("active")}>
          {showSettings ? (
            <Settings onClose={() => setShowSettings(false)} />
          ) : selectedStatusFeature ? (
            <StatusExpandedView />
          ) : notification ? (
            <NotificationExpandedView />
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 12, alignItems: "center", justifyContent: "center", height: "100%", minHeight: 160 }}>
              <span style={{ fontSize: 14, opacity: 0.6, color: "var(--pill-text)" }}>No activity</span>
              <div style={{ display: "flex", gap: 8 }}>
                <button
                  onClick={() => transitionTo("active")}
                  style={{
                    padding: "6px 16px",
                    borderRadius: 8,
                    border: "1px solid var(--pill-border)",
                    background: "rgba(255,255,255,0.05)",
                    color: "var(--pill-text)",
                    fontSize: 12,
                    cursor: "pointer",
                  }}
                >
                  Collapse
                </button>
                <button
                  onClick={() => setShowSettings(true)}
                  style={{
                    padding: "6px 16px",
                    borderRadius: 8,
                    border: "1px solid var(--pill-border)",
                    background: "rgba(255,255,255,0.05)",
                    color: "var(--pill-text)",
                    fontSize: 12,
                    cursor: "pointer",
                  }}
                >
                  Settings
                </button>
              </div>
            </div>
          )}
        </ExpandedView>

        <ContextMenu
          isOpen={menuOpen}
          items={contextMenuItems}
          onClose={() => setMenuOpen(false)}
        />

        <ConnectionIndicator />
      </div>
    </ThemeProvider>
  );
}

export default App;
