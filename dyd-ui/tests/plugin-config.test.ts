import { test } from "node:test";
import assert from "node:assert/strict";
import {
  schemaToFields,
  defaultsFor,
  validateRequired,
  debounce,
} from "../src/lib/plugin-config.ts";

const SCHEMA = {
  type: "object",
  required: ["api_key"],
  properties: {
    api_key: { type: "string", title: "API Key", description: "Secret token" },
    poll_interval: { type: "integer", default: 10 },
    verbose: { type: "boolean", default: false },
    level: { type: "string", enum: ["low", "high"], default: "low" },
    tags: { type: "array", items: { type: "string" } },
    readonly_id: { type: "string", readOnly: true },
    nested: {
      type: "object",
      properties: { host: { type: "string" }, port: { type: "number" } },
    },
  },
};

test("schemaToFields maps each JSON-Schema type to a field kind", () => {
  const fields = schemaToFields(SCHEMA);
  const byKey = Object.fromEntries(fields.map((f) => [f.key, f]));
  assert.equal(byKey.api_key.kind, "string");
  assert.equal(byKey.poll_interval.kind, "number");
  assert.equal(byKey.verbose.kind, "boolean");
  assert.equal(byKey.level.kind, "enum");
  assert.deepEqual(byKey.level.options, ["low", "high"]);
  assert.equal(byKey.tags.kind, "array");
  assert.equal(byKey.nested.kind, "object");
});

test("schemaToFields surfaces label, description, required, readOnly", () => {
  const byKey = Object.fromEntries(schemaToFields(SCHEMA).map((f) => [f.key, f]));
  assert.equal(byKey.api_key.label, "API Key");
  assert.equal(byKey.api_key.description, "Secret token");
  assert.equal(byKey.api_key.required, true);
  assert.equal(byKey.poll_interval.required, false);
  assert.equal(byKey.readonly_id.readOnly, true);
});

test("schemaToFields humanizes missing titles and recurses objects", () => {
  const byKey = Object.fromEntries(schemaToFields(SCHEMA).map((f) => [f.key, f]));
  assert.equal(byKey.poll_interval.label, "Poll interval");
  assert.equal(byKey.nested.fields?.length, 2);
  assert.equal(byKey.nested.fields?.[0].key, "host");
});

test("schemaToFields returns [] for absent/invalid schema", () => {
  assert.deepEqual(schemaToFields(null), []);
  assert.deepEqual(schemaToFields({}), []);
  assert.deepEqual(schemaToFields("nope"), []);
});

test("defaultsFor collects declared defaults, including nested objects", () => {
  const defaults = defaultsFor(schemaToFields(SCHEMA));
  assert.equal(defaults.poll_interval, 10);
  assert.equal(defaults.verbose, false);
  assert.equal(defaults.level, "low");
  assert.deepEqual(defaults.nested, {});
});

test("validateRequired flags empty required fields only", () => {
  const fields = schemaToFields(SCHEMA);
  assert.deepEqual(validateRequired(fields, {}), ["api_key"]);
  assert.deepEqual(validateRequired(fields, { api_key: "" }), ["api_key"]);
  assert.deepEqual(validateRequired(fields, { api_key: "x" }), []);
});

test("debounce fires once after the delay with the latest args", async () => {
  let calls: number[] = [];
  const fn = debounce((n: number) => calls.push(n), 20);
  fn(1);
  fn(2);
  fn(3);
  assert.deepEqual(calls, []);
  await new Promise((r) => setTimeout(r, 40));
  assert.deepEqual(calls, [3]);
});

test("debounce cancel drops a pending call", async () => {
  let calls = 0;
  const fn = debounce(() => (calls += 1), 20);
  fn();
  fn.cancel();
  await new Promise((r) => setTimeout(r, 40));
  assert.equal(calls, 0);
});
