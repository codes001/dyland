import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "motion/react";
import { listen } from "@tauri-apps/api/event";
import Pill from "./components/Pill";
import ExpandedView from "./components/ExpandedView";
import ContextMenu from "./components/ContextMenu";
import { ConnectionIndicator } from "./components/ConnectionIndicator";
import StatusIconRow from "./components/status/StatusIconRow";
import StatusExpandedView from "./components/status/StatusExpandedView";
import PillNotificationContent from "./components/notification/PillNotificationContent";
import NotificationExpandedView from "./components/notification/NotificationExpandedView";
import Settings from "./components/settings/Settings";
import { connectionStore } from "./stores/connection";
import { usePillStore } from "./stores/pill-store";
import { useActivityStore } from "./stores/activity-store";
import { useMediaStore } from "./stores/media-store";
import { useNotificationStore } from "./stores/notification-store";
import { useSystemStore } from "./stores/system-store";


function App() {
  const state = usePillStore((s) => s.state);
  const isFullscreen = usePillStore((s) => s.isFullscreen);
  const transitionTo = usePillStore((s) => s.transitionTo);
  const currentActivity = useActivityStore((s) => s.currentActivity);
  const media = useMediaStore((s) => s.media);
  const notification = useNotificationStore((s) => s.notification);
  const selectedStatusFeature = useSystemStore((s) => s.selectedStatusFeature);
  const [showSettings, setShowSettings] = useState(false);

  useEffect(() => {
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

  const hasContent = currentActivity !== null || media !== null || notification !== null;

  return (
    <div
      className="flex h-screen w-screen items-start justify-center"
      onClick={() => {
        if (state === "expanded") transitionTo("active");
      }}
    >
      <Pill>
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
                  <span style={{ color: "white", fontSize: 12, opacity: 0.4 }}>dyd</span>
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
            <span style={{ fontSize: 14, opacity: 0.6 }}>No activity</span>
            <div style={{ display: "flex", gap: 8 }}>
              <button
                onClick={() => transitionTo("active")}
                style={{
                  padding: "6px 16px",
                  borderRadius: 8,
                  border: "1px solid rgba(255,255,255,0.2)",
                  background: "rgba(255,255,255,0.05)",
                  color: "#fff",
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
                  border: "1px solid rgba(255,255,255,0.2)",
                  background: "rgba(255,255,255,0.05)",
                  color: "#fff",
                  fontSize: 12,
                  cursor: "pointer",
                }}
              >
                Plugins
              </button>
            </div>
          </div>
        )}
      </ExpandedView>

      <ContextMenu
        isOpen={false}
        items={[]}
        onClose={() => {}}
      />

      <ConnectionIndicator />
    </div>
  );
}

export default App;
