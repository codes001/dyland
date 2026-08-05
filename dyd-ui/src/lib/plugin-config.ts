// Pure logic behind PluginConfigForm (story 6-5, US-6.5.3).
//
// Kept free of React so it can be unit-tested directly: JSON-Schema →
// field-descriptor generation, a debounce helper, required-field validation,
// and default extraction. The component is a thin renderer over these.

/** A single form control derived from one JSON-Schema property. */
export interface FieldDescriptor {
  key: string;
  label: string;
  description?: string;
  kind: "string" | "number" | "boolean" | "enum" | "array" | "object";
  required: boolean;
  readOnly: boolean;
  /** Present for `kind === "enum"`. */
  options?: string[];
  /** Schema-declared default, if any. */
  default?: unknown;
  /** Nested descriptors for `kind === "object"`. */
  fields?: FieldDescriptor[];
}

interface JsonSchema {
  type?: string | string[];
  title?: string;
  description?: string;
  properties?: Record<string, JsonSchema>;
  required?: string[];
  enum?: unknown[];
  default?: unknown;
  readOnly?: boolean;
  items?: JsonSchema;
}

function humanize(key: string): string {
  const spaced = key.replace(/[_-]+/g, " ").replace(/([a-z])([A-Z])/g, "$1 $2");
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}

function firstType(schema: JsonSchema): string | undefined {
  const t = schema.type;
  return Array.isArray(t) ? t.find((x) => x !== "null") : t;
}

function fieldKind(schema: JsonSchema): FieldDescriptor["kind"] {
  if (Array.isArray(schema.enum) && schema.enum.length > 0) return "enum";
  switch (firstType(schema)) {
    case "boolean":
      return "boolean";
    case "number":
    case "integer":
      return "number";
    case "array":
      return "array";
    case "object":
      return "object";
    default:
      return "string";
  }
}

/**
 * Turn a plugin `config_schema` (a JSON-Schema object with `properties`) into an
 * ordered list of field descriptors. Unknown/absent schemas yield `[]`.
 * Nested objects recurse one property level at a time.
 */
export function schemaToFields(schema: unknown): FieldDescriptor[] {
  if (!schema || typeof schema !== "object") return [];
  const root = schema as JsonSchema;
  const props = root.properties;
  if (!props || typeof props !== "object") return [];
  const requiredSet = new Set(root.required ?? []);

  return Object.entries(props).map(([key, prop]) => {
    const kind = fieldKind(prop);
    const descriptor: FieldDescriptor = {
      key,
      label: prop.title ?? humanize(key),
      description: prop.description,
      kind,
      required: requiredSet.has(key),
      readOnly: prop.readOnly === true,
      default: prop.default,
    };
    if (kind === "enum") {
      descriptor.options = (prop.enum ?? []).map((v) => String(v));
    }
    if (kind === "object") {
      descriptor.fields = schemaToFields(prop);
    }
    return descriptor;
  });
}

/** Collect the `default` values declared across a set of descriptors. */
export function defaultsFor(fields: FieldDescriptor[]): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const f of fields) {
    if (f.kind === "object" && f.fields) {
      out[f.key] = defaultsFor(f.fields);
    } else if (f.default !== undefined) {
      out[f.key] = f.default;
    }
  }
  return out;
}

/**
 * Validate that every required field has a non-empty value. Returns the list of
 * offending keys (empty ⇒ valid). Empty string, null, undefined, and empty
 * arrays count as missing.
 */
export function validateRequired(
  fields: FieldDescriptor[],
  values: Record<string, unknown>,
): string[] {
  const missing: string[] = [];
  for (const f of fields) {
    if (!f.required) continue;
    const v = values[f.key];
    const empty =
      v === undefined ||
      v === null ||
      v === "" ||
      (Array.isArray(v) && v.length === 0);
    if (empty) missing.push(f.key);
  }
  return missing;
}

/**
 * Trailing-edge debounce. Returns a wrapper that delays `fn` until `delayMs`
 * has elapsed since the last call, plus a `cancel()` to drop a pending call.
 */
export function debounce<A extends unknown[]>(
  fn: (...args: A) => void,
  delayMs: number,
): { (...args: A): void; cancel: () => void } {
  let timer: ReturnType<typeof setTimeout> | null = null;
  const wrapped = (...args: A) => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      timer = null;
      fn(...args);
    }, delayMs);
  };
  wrapped.cancel = () => {
    if (timer) {
      clearTimeout(timer);
      timer = null;
    }
  };
  return wrapped;
}
