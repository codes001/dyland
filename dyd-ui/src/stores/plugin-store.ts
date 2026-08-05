import { create } from "zustand";
import type { PluginInfo } from "../types/plugin";

export interface PluginStore {
  plugins: PluginInfo[];
  selectedName: string | null;

  /** Replace the whole list (from a `PluginList` snapshot). */
  setPlugins: (plugins: PluginInfo[]) => void;
  /** Merge a single plugin's health (from a real-time `PluginChanged` event). */
  updatePlugin: (plugin: PluginInfo) => void;
  /** Remove a plugin locally (optimistic uninstall). */
  removePlugin: (name: string) => void;
  /** Select a plugin by name (opens the detail view); null closes it. */
  setSelected: (name: string | null) => void;
}

export const usePluginStore = create<PluginStore>((set, get) => ({
  plugins: [],
  selectedName: null,

  setPlugins: (plugins: PluginInfo[]) => {
    set({ plugins });
    // Drop a stale selection if that plugin is gone from the snapshot.
    const { selectedName } = get();
    if (selectedName && !plugins.some((p) => p.name === selectedName)) {
      set({ selectedName: null });
    }
  },

  updatePlugin: (plugin: PluginInfo) => {
    const { plugins } = get();
    const idx = plugins.findIndex((p) => p.name === plugin.name);
    if (idx === -1) {
      set({ plugins: [...plugins, plugin] });
    } else {
      const next = plugins.slice();
      next[idx] = plugin;
      set({ plugins: next });
    }
  },

  removePlugin: (name: string) => {
    const { plugins, selectedName } = get();
    set({
      plugins: plugins.filter((p) => p.name !== name),
      selectedName: selectedName === name ? null : selectedName,
    });
  },

  setSelected: (name: string | null) => set({ selectedName: name }),
}));
