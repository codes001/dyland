export type Urgency = "low" | "normal" | "critical";

export interface ResolvedIcon {
  kind: "image";
  src: string;
}

export interface LetterIcon {
  kind: "letter";
  letter: string;
  appName: string;
}

export interface BellIcon {
  kind: "bell";
  appName: string;
}

export type AppIcon = ResolvedIcon | LetterIcon | BellIcon;

const IS_PATH = /^(file:|[a-zA-Z]:[\\/]|\/|\.\.?\/)/i;

export function normalizeUrgency(value: string | null | undefined): Urgency {
  switch (value) {
    case "low":
      return "low";
    case "critical":
      return "critical";
    default:
      return "normal";
  }
}

export function truncateSummary(text: string | null | undefined, max = 25): string {
  const value = text?.trim();
  if (!value) return "";
  const chars = Array.from(value);
  if (chars.length <= max) return value;
  return `${chars.slice(0, Math.max(0, max - 1)).join("").trimEnd()}…`;
}

export function displayText(
  notification: { summary?: string | null; app_name?: string } | null | undefined,
): string {
  if (!notification) return "";
  const summary = notification.summary?.trim();
  if (summary) return summary;
  return notification.app_name?.trim() ?? "";
}

function letterFor(appName: string | null | undefined): string {
  const name = appName?.trim();
  if (!name) return "?";
  return name[0].toUpperCase();
}

export function resolveAppIcon(
  appIcon: string | null | undefined,
  appName: string | null | undefined,
): AppIcon {
  const icon = appIcon?.trim();
  if (icon && IS_PATH.test(icon)) {
    return { kind: "image", src: icon };
  }
  return { kind: "letter", letter: letterFor(appName), appName: appName ?? "" };
}

export function relativeTime(timestamp: number | null | undefined, now = Date.now()): string {
  if (typeof timestamp !== "number" || !Number.isFinite(timestamp)) return "";
  const diffMs = now - timestamp * 1000;
  if (!Number.isFinite(diffMs)) return "";
  const rtf = new Intl.RelativeTimeFormat("en", { numeric: "auto" });
  const seconds = Math.max(0, Math.round(diffMs / 1000));
  if (seconds < 60) return rtf.format(0, "second");
  const minutes = Math.max(0, Math.round(seconds / 60));
  if (minutes < 60) return rtf.format(-minutes, "minute");
  const hours = Math.max(0, Math.round(minutes / 60));
  if (hours < 24) return rtf.format(-hours, "hour");
  const days = Math.max(0, Math.round(hours / 24));
  return rtf.format(-days, "day");
}

const ICON_CACHE_LIMIT = 200;
const iconCache = new Map<string, AppIcon>();

function cacheKey(appIcon: string | null | undefined, appName: string | null | undefined): string {
  return `${appIcon ?? ""}|${appName ?? ""}`;
}

export function resolveAppIconCached(
  appIcon: string | null | undefined,
  appName: string | null | undefined,
): AppIcon {
  const key = cacheKey(appIcon, appName);
  const cached = iconCache.get(key);
  if (cached) return cached;
  const resolved = resolveAppIcon(appIcon, appName);
  iconCache.set(key, resolved);
  if (iconCache.size > ICON_CACHE_LIMIT) {
    const oldest = iconCache.keys().next().value as string | undefined;
    if (oldest !== undefined) iconCache.delete(oldest);
  }
  return resolved;
}

export function evictIcon(appIcon: string | null | undefined, appName: string | null | undefined): void {
  iconCache.delete(cacheKey(appIcon, appName));
}

export function clearIconCache(): void {
  iconCache.clear();
}