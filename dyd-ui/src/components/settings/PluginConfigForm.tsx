import { useEffect, useMemo, useRef, useState } from "react";
import {
  debounce,
  defaultsFor,
  schemaToFields,
  validateRequired,
  type FieldDescriptor,
} from "../../lib/plugin-config";
import { ipcClient } from "../../ipc/client";

const AUTOSAVE_DEBOUNCE_MS = 1000;

interface Props {
  pluginName: string;
  schema: unknown;
  config: unknown;
}

const inputStyle: React.CSSProperties = {
  width: "100%",
  padding: "6px 8px",
  borderRadius: 6,
  border: "1px solid rgba(255,255,255,0.2)",
  background: "rgba(255,255,255,0.05)",
  color: "#fff",
  fontSize: 12,
};

/**
 * Auto-generated config form from a plugin's `config_schema`. Renders string /
 * number / boolean / enum / array / object fields, auto-saves changes after a
 * 1s debounce, validates required fields before saving, and displays (but does
 * not allow editing of) `readOnly` fields.
 */
export default function PluginConfigForm({ pluginName, schema, config }: Props) {
  const fields = useMemo(() => schemaToFields(schema), [schema]);

  const initialValues = useMemo(() => {
    const base = defaultsFor(fields);
    return { ...base, ...(config && typeof config === "object" ? config : {}) };
  }, [fields, config]);

  const [values, setValues] = useState<Record<string, unknown>>(initialValues);
  const [missing, setMissing] = useState<string[]>([]);

  // Re-seed when the selected plugin (or its incoming config) changes.
  useEffect(() => {
    setValues(initialValues);
    setMissing([]);
  }, [initialValues]);

  const save = useRef(
    debounce((name: string, next: Record<string, unknown>) => {
      void ipcClient.setPluginConfig(name, next);
    }, AUTOSAVE_DEBOUNCE_MS),
  );

  useEffect(() => {
    const debounced = save.current;
    return () => debounced.cancel();
  }, []);

  if (fields.length === 0) {
    return (
      <p style={{ fontSize: 12, opacity: 0.5, margin: 0 }}>
        This plugin has no configurable options.
      </p>
    );
  }

  const commit = (next: Record<string, unknown>) => {
    setValues(next);
    const bad = validateRequired(fields, next);
    setMissing(bad);
    if (bad.length === 0) {
      save.current(pluginName, next);
    }
  };

  const setField = (key: string, value: unknown) => commit({ ...values, [key]: value });

  const resetDefaults = () => {
    save.current.cancel();
    const defaults = defaultsFor(fields);
    setValues(defaults);
    setMissing(validateRequired(fields, defaults));
    void ipcClient.setPluginConfig(pluginName, defaults);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      {fields.map((f) => (
        <Field
          key={f.key}
          field={f}
          value={values[f.key]}
          invalid={missing.includes(f.key)}
          onChange={(v) => setField(f.key, v)}
        />
      ))}
      <button
        onClick={resetDefaults}
        style={{
          alignSelf: "flex-start",
          padding: "6px 12px",
          borderRadius: 6,
          border: "1px solid rgba(255,255,255,0.2)",
          background: "transparent",
          color: "#fff",
          fontSize: 12,
          cursor: "pointer",
        }}
      >
        Reset to defaults
      </button>
    </div>
  );
}

interface FieldProps {
  field: FieldDescriptor;
  value: unknown;
  invalid: boolean;
  onChange: (value: unknown) => void;
}

function Field({ field, value, invalid, onChange }: FieldProps) {
  const disabled = field.readOnly;

  return (
    <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      <span style={{ fontSize: 12, fontWeight: 600 }}>
        {field.label}
        {field.required && <span style={{ color: "#d29922" }}> *</span>}
        {field.readOnly && (
          <span style={{ opacity: 0.5, fontWeight: 400 }}> (read-only)</span>
        )}
      </span>
      {field.description && (
        <span style={{ fontSize: 11, opacity: 0.6 }}>{field.description}</span>
      )}
      <Control field={field} value={value} disabled={disabled} onChange={onChange} />
      {invalid && (
        <span style={{ fontSize: 11, color: "#f85149" }}>This field is required.</span>
      )}
    </label>
  );
}

function Control({
  field,
  value,
  disabled,
  onChange,
}: {
  field: FieldDescriptor;
  value: unknown;
  disabled: boolean;
  onChange: (value: unknown) => void;
}) {
  switch (field.kind) {
    case "boolean":
      return (
        <input
          type="checkbox"
          disabled={disabled}
          checked={value === true}
          onChange={(e) => onChange(e.target.checked)}
          style={{ alignSelf: "flex-start" }}
        />
      );
    case "number":
      return (
        <input
          type="number"
          disabled={disabled}
          value={value === undefined || value === null ? "" : String(value)}
          onChange={(e) => onChange(e.target.value === "" ? undefined : Number(e.target.value))}
          style={inputStyle}
        />
      );
    case "enum":
      return (
        <select
          disabled={disabled}
          value={value === undefined || value === null ? "" : String(value)}
          onChange={(e) => onChange(e.target.value)}
          style={inputStyle}
        >
          <option value="">—</option>
          {(field.options ?? []).map((opt) => (
            <option key={opt} value={opt}>
              {opt}
            </option>
          ))}
        </select>
      );
    case "array":
      return (
        <input
          type="text"
          disabled={disabled}
          placeholder="comma,separated,values"
          value={Array.isArray(value) ? value.join(", ") : ""}
          onChange={(e) =>
            onChange(
              e.target.value
                .split(",")
                .map((s) => s.trim())
                .filter((s) => s.length > 0),
            )
          }
          style={inputStyle}
        />
      );
    case "object":
      return (
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: 10,
            paddingLeft: 12,
            borderLeft: "1px solid rgba(255,255,255,0.1)",
          }}
        >
          {(field.fields ?? []).map((child) => {
            const obj = (value && typeof value === "object" ? value : {}) as Record<
              string,
              unknown
            >;
            return (
              <Field
                key={child.key}
                field={child}
                value={obj[child.key]}
                invalid={false}
                onChange={(v) => onChange({ ...obj, [child.key]: v })}
              />
            );
          })}
        </div>
      );
    default:
      return (
        <input
          type="text"
          disabled={disabled}
          value={value === undefined || value === null ? "" : String(value)}
          onChange={(e) => onChange(e.target.value)}
          style={inputStyle}
        />
      );
  }
}
