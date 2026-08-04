import { test } from "node:test";
import assert from "node:assert/strict";
import {
  clearIconCache,
  displayText,
  normalizeUrgency,
  relativeTime,
  resolveAppIcon,
  resolveAppIconCached,
  truncateSummary,
  type AppIcon,
} from "../src/lib/notification-utils.ts";

test("normalizeUrgency maps known values", () => {
  assert.equal(normalizeUrgency("low"), "low");
  assert.equal(normalizeUrgency("normal"), "normal");
  assert.equal(normalizeUrgency("critical"), "critical");
});

test("normalizeUrgency falls back to normal for unknown/null", () => {
  assert.equal(normalizeUrgency(null), "normal");
  assert.equal(normalizeUrgency(undefined), "normal");
  assert.equal(normalizeUrgency("urgent"), "normal");
  assert.equal(normalizeUrgency(""), "normal");
});

test("truncateSummary leaves short text untouched", () => {
  assert.equal(truncateSummary("hello world"), "hello world");
  assert.equal(truncateSummary(""), "");
  assert.equal(truncateSummary(null), "");
});

test("truncateSummary truncates long text with ellipsis", () => {
  const long = "A".repeat(40);
  const out = truncateSummary(long);
  assert.equal(out.length, 25);
  assert.equal(out.endsWith("…"), true);
});

test("truncateSummary respects custom max", () => {
  const out = truncateSummary("abcdefgh", 5);
  assert.equal(out, "abcd…");
});

test("displayText prefers summary over app name", () => {
  assert.equal(displayText({ summary: "Hi", app_name: "Slack" }), "Hi");
  assert.equal(displayText({ summary: "   ", app_name: "Slack" }), "Slack");
  assert.equal(displayText({ summary: null, app_name: "Slack" }), "Slack");
  assert.equal(displayText(null), "");
});

test("resolveAppIcon returns image for local path/url icons", () => {
  const a = resolveAppIcon("/usr/share/icons/app.png", "Slack") as AppIcon;
  assert.equal(a.kind, "image");
  assert.equal((a as { src: string }).src, "/usr/share/icons/app.png");
  const b = resolveAppIcon("file:///tmp/icon.png", "Slack");
  assert.equal(b.kind, "image");
  const win = resolveAppIcon("C:\\icons\\app.png", "Slack");
  assert.equal(win.kind, "image");
  const rel = resolveAppIcon("./icons/app.png", "Slack");
  assert.equal(rel.kind, "image");
});

test("resolveAppIcon rejects remote http(s) icon URLs (privacy)", () => {
  const remote = resolveAppIcon("https://example.com/icon.png", "Slack");
  assert.equal(remote.kind, "letter");
});

test("resolveAppIcon falls back to letter for icon names/null", () => {
  const iconName = resolveAppIcon("com.slack.Slack", "Slack");
  assert.equal(iconName.kind, "letter");
  assert.equal((iconName as { letter: string }).letter, "S");

  const none = resolveAppIcon(null, "Telegram");
  assert.equal(none.kind, "letter");
  assert.equal((none as { letter: string }).letter, "T");
});

test("resolveAppIcon upper-cases first letter and handles empty name", () => {
  const upper = resolveAppIcon(null, "discord");
  assert.equal((upper as { letter: string }).letter, "D");
  const empty = resolveAppIcon(null, "");
  assert.equal((empty as { letter: string }).letter, "?");
});

test("resolveAppIconCached caches by appIcon|appName", () => {
  clearIconCache();
  const first = resolveAppIconCached("/a.png", "A");
  const second = resolveAppIconCached("/a.png", "A");
  assert.equal(first.kind, "image");
  assert.equal(second, first); // same cached reference
  clearIconCache();
  const afterClear = resolveAppIconCached("/a.png", "A");
  assert.notEqual(afterClear, first);
});

test("relativeTime returns empty for invalid timestamp", () => {
  assert.equal(relativeTime(null), "");
  assert.equal(relativeTime(undefined), "");
  assert.equal(relativeTime(Number.NaN), "");
});

test("relativeTime formats past timestamps", () => {
  const now = 1_000_000_000_000;
  const minuteAgoSec = (now - 60_000) / 1000;
  const out = relativeTime(minuteAgoSec, now);
  assert.ok(out.length > 0);
  assert.match(out, /minute|min|min\./i);
});

test("relativeTime shows seconds for under a minute", () => {
  const now = 1_000_000_000_000;
  const recentlySec = (now - 10_000) / 1000;
  const out = relativeTime(recentlySec, now);
  assert.match(out, /sec|second|s ago|/i);
});

test("relativeTime clamps future timestamps to now", () => {
  const now = 1_000_000_000_000;
  const futureSec = (now + 3_600_000) / 1000;
  const out = relativeTime(futureSec, now);
  assert.equal(out, "now");
});