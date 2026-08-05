import { create } from "zustand";
import type { AppConfig } from "../../../types/src/index";
import { ipcClient } from "../ipc/client";

export const DEFAULT_CONFIG: AppConfig = {
  general: {
    start_on_login: false,
  },
  appearance: {
    theme: "dark",
  },
  accessibility: {
    reduce_motion: null,
    high_contrast: null,
    font_scale: null,
  },
  features: {
    battery: true,
    wifi: true,
    bluetooth: true,
    volume: true,
    brightness: true,
    media: true,
    notifications: true,
  },
  notifications: {
    dnd_enabled: false,
    app_rules: {},
  },
  shortcuts: {
    bindings: {
      expand: "Space",
      collapse: "Escape",
      prev_activity: "ArrowLeft",
      next_activity: "ArrowRight",
      toggle_dnd: "d",
      open_settings: "s",
    },
  },
  last_category: null,
};

interface ConfigState {
  config: AppConfig;
  loaded: boolean;
  setConfig: (config: AppConfig) => void;
  updateConfig: (patch: Record<string, unknown>) => Promise<void>;
  resetToDefaults: () => Promise<void>;
}

export const useConfigStore = create<ConfigState>((set, get) => ({
  config: DEFAULT_CONFIG,
  loaded: false,
  setConfig: (config) => set({ config, loaded: true }),
  updateConfig: async (patch) => {
    // Deep merge patch into current config for optimistic UI update
    const current = get().config;
    const patchObj = patch as Record<string, unknown>;
    const patchNotifs = patchObj.notifications as Record<string, unknown> | undefined;
    const patchShortcuts = patchObj.shortcuts as Record<string, unknown> | undefined;

    const nextConfig = {
      ...current,
      ...patchObj,
      general: { ...current.general, ...((patchObj.general as object) || {}) },
      appearance: { ...current.appearance, ...((patchObj.appearance as object) || {}) },
      accessibility: { ...current.accessibility, ...((patchObj.accessibility as object) || {}) },
      features: { ...current.features, ...((patchObj.features as object) || {}) },
      notifications: {
        ...current.notifications,
        ...(patchNotifs || {}),
        app_rules: {
          ...(current.notifications?.app_rules || {}),
          ...((patchNotifs?.app_rules as object) || {}),
        },
      },
      shortcuts: {
        ...current.shortcuts,
        ...(patchShortcuts || {}),
        bindings: {
          ...(current.shortcuts?.bindings || {}),
          ...((patchShortcuts?.bindings as object) || {}),
        },
      },
    } as AppConfig;

    set({ config: nextConfig });
    await ipcClient.updateConfig(patch);
  },
  resetToDefaults: async () => {
    set({ config: DEFAULT_CONFIG });
    await ipcClient.updateConfig(DEFAULT_CONFIG as unknown as Record<string, unknown>);
  },
}));
