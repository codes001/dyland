import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import Pill from "./components/Pill";
import ExpandedView from "./components/ExpandedView";
import ContextMenu from "./components/ContextMenu";
import { ConnectionIndicator } from "./components/ConnectionIndicator";
import { connectionStore } from "./stores/connection";
import { usePillStore } from "./stores/pill-store";

function App() {
  const state = usePillStore((s) => s.state);
  const isFullscreen = usePillStore((s) => s.isFullscreen);
  const transitionTo = usePillStore((s) => s.transitionTo);

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

  return (
    <div
      className="flex h-screen w-screen items-start justify-center"
      onClick={() => {
        if (state === "expanded") transitionTo("active");
      }}
    >
      <Pill>
        {state === "dormant" && !isFullscreen && (
          <span style={{ color: "white", fontSize: 12, opacity: 0.4 }}>dyd</span>
        )}
      </Pill>

      <ExpandedView onClose={() => transitionTo("active")}>
        <div style={{ display: "flex", flexDirection: "column", gap: 12, alignItems: "center", justifyContent: "center", height: "100%", minHeight: 160 }}>
          <span style={{ fontSize: 14, opacity: 0.6 }}>No activity</span>
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
        </div>
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
