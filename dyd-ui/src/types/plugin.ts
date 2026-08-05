// Plugin management types for the Settings UI (story 6-5).
//
// Mirrors the daemon's `PluginHealthPayload` (types/src/index.ts) and adds
// presentation helpers — a status→badge-color map used by the plugin list and
// detail views. Kept separate from the wire type so UI concerns don't leak into
// the shared IPC contract.

import type { PluginHealthPayload } from "../../../types/src/index";

/** Wire shape as delivered by the daemon (re-exported for local ergonomics). */
export type PluginInfo = PluginHealthPayload;

/** Alias used by detail views that focus on the health/runtime fields. */
export type PluginHealth = PluginHealthPayload;

/** Visual severity a badge can render. */
export type BadgeColor = "green" | "yellow" | "gray" | "blue";

/**
 * Map a plugin's effective status to a badge color.
 *
 * The `disabled` flag wins over the runtime `state` (a disabled plugin is gray
 * regardless of its last-known state), matching AC1: running=green, error=yellow,
 * disabled=gray. Transient/among-us states fall back to a neutral gray.
 */
export function badgeColor(plugin: Pick<PluginInfo, "state" | "disabled">): BadgeColor {
  if (plugin.disabled) return "gray";
  switch (plugin.state) {
    case "running":
      return "green";
    case "error":
      return "yellow";
    case "loaded":
      return "blue";
    case "stopped":
    case "unloaded":
      return "gray";
    default:
      return "gray";
  }
}

/** Human-facing label for a plugin's effective status. */
export function statusLabel(plugin: Pick<PluginInfo, "state" | "disabled">): string {
  if (plugin.disabled) return "Disabled";
  if (!plugin.state) return "Unknown";
  return plugin.state.charAt(0).toUpperCase() + plugin.state.slice(1);
}

/** Hex color for a badge, so components don't each re-derive the palette. */
export const BADGE_HEX: Record<BadgeColor, string> = {
  green: "#3fb950",
  yellow: "#d29922",
  gray: "#6e7681",
  blue: "#58a6ff",
};
