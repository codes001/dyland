import { test } from "node:test";
import assert from "node:assert/strict";
import { badgeColor, statusLabel, BADGE_HEX } from "../src/types/plugin.ts";

test("badgeColor maps running to green", () => {
  assert.equal(badgeColor({ state: "running", disabled: false }), "green");
});

test("badgeColor maps error to yellow", () => {
  assert.equal(badgeColor({ state: "error", disabled: false }), "yellow");
});

test("disabled overrides state to gray", () => {
  assert.equal(badgeColor({ state: "running", disabled: true }), "gray");
  assert.equal(badgeColor({ state: "error", disabled: true }), "gray");
});

test("stopped/unloaded/unknown states are gray", () => {
  assert.equal(badgeColor({ state: "stopped", disabled: false }), "gray");
  assert.equal(badgeColor({ state: "unloaded", disabled: false }), "gray");
  assert.equal(badgeColor({ state: "who-knows", disabled: false }), "gray");
});

test("loaded state is blue", () => {
  assert.equal(badgeColor({ state: "loaded", disabled: false }), "blue");
});

test("every badge color has a hex", () => {
  for (const c of ["green", "yellow", "gray", "blue"] as const) {
    assert.match(BADGE_HEX[c], /^#[0-9a-f]{6}$/i);
  }
});

test("statusLabel capitalizes state and prefers Disabled", () => {
  assert.equal(statusLabel({ state: "running", disabled: false }), "Running");
  assert.equal(statusLabel({ state: "running", disabled: true }), "Disabled");
  assert.equal(statusLabel({ state: "", disabled: false }), "Unknown");
});
