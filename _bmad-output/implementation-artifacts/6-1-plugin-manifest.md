# Story 6.1: Plugin Manifest Format

Status: review

## Story

As a developer (plugin author),
I want a clear, validated plugin manifest format describing metadata, permissions, entry points, and UI components,
so that I can author plugins the daemon can safely load and validate.

## Acceptance Criteria

1. Manifest format is JSON with a `schema_version` field for future compatibility (US-6.1.1)
2. Required fields: `name`, `version`, `author`, `entry_point` — missing required field produces a validation error naming the field (US-6.1.1/6.1.2)
3. Optional fields supported: `description`, `homepage`, `license`, `permissions`, `ui_components`, `dependencies`, `config_schema` (US-6.1.1)
4. `permissions` is an array of D-Bus services, system resources, or API features the plugin needs (US-6.1.1)
5. `entry_point` is a WASM binary path relative to the plugin directory (US-6.1.1)
6. Unknown/invalid permission names produce a warning and are ignored (forward-compatible, not rejected) (US-6.1.1/6.1.2)
7. Non-semver `version` string produces a warning and is best-effort parsed (US-6.1.1)
8. Validation checks required fields, field types, permission names, and version parseability (US-6.1.2)
9. Validation errors are returned with field path and reason (US-6.1.2)
10. Validation runs on install and on daemon startup for installed plugins; invalid plugins are skipped with an error log (US-6.1.2)

## Tasks / Subtasks

- [x] Define manifest data types in Rust (AC: 1, 2, 3, 4, 5)
  - [x] Add workspace library crate `dyd-plugin-types` (or reuse a module) holding `PluginManifest`, `PluginPermission`, `UiComponent` structs derived with serde Serialize/Deserialize
  - [x] Include `schema_version: u32`, `name: String`, `version: String`, `author: String`, `entry_point: String`
  - [x] Optionals: `description`, `homepage`, `license`, `permissions: Vec<String>`, `ui_components: Vec<String>`, `dependencies: Vec<String>`, `config_schema: serde_json::Value`
  - [x] Define recognized permission enumerations for D-Bus services, unix sockets, http endpoints, and API features
- [x] Implement semantic validation logic (AC: 2, 6, 7, 8, 9)
  - [x] Parse manifest with serde; on missing required field, return error carrying the field path
  - [x] Validate `version`; non-semver yields a warning and best-effort parse
  - [x] Validate `permissions`; unknown permissions yield a warning (kept in manifest, flagged)
  - [x] Return structured `ManifestValidation` result with `errors: Vec<ValidationIssue>` and `warnings: Vec<ValidationIssue>`
  - [x] Provide `validate_manifest(json: &str) -> Result<PluginManifest, ValidationReport>`
- [x] Add unit tests (AC: 2, 6, 7, 8, 9)
  - [x] Test valid full manifest parses cleanly with zero errors/warnings
  - [x] Test missing required field (name/version/author/entry_point) errors with the field name
  - [x] Test unknown permission produces a warning but not an error
  - [x] Test non-semver version produces a warning but parses best-effort
  - [x] Test invalid JSON fails parse with reason
  - [x] Test `config_schema` passthrough is preserved as JSON

## Dev Notes

- This story is Feature 6.1 in `bmad-output/EPIC-06-PLUGIN-SYSTEM.md`. Keep the format simple to avoid over-engineering (epic line 44).
- Both a Rust (daemon) and TypeScript (zod) schema were specified in the epic; the daemon-side Rust schema is the authoritative one for runtime validation. Zod UI validation is deferred to the management-UI story (6-5) to keep this story focused.
- We do NOT yet wire manifests into daemon load/startup; that is Feature-2 (6-2 lifecycle). This story is design + daemon validation only.
- Follow the existing `serde` + `serde_json` patterns already used across `types/src/lib.rs` and `dyd-daemon`.
- Workspace members are defined in `Cargo.toml`; `types/` is the shared-types crate `dyd-types`. New Rust types for plugins may live in the existing `types` crate to match the established pattern.
- Exploration showed no plugin-source file exists yet; the daemon uses per-source `dyd-daemon/src/sources/*.rs`. Plugin modules will be added to the daemon crate (`dyd-daemon/src/plugins/`) in the lifecycle story.

### Project Structure Notes

- New file: `types/src/plugin.rs` (PluginManifest + validation) exported from `types/src/lib.rs`
- Unit tests inline in `types/src/plugin.rs`
- No UI changes; daemon wiring deferred to 6-2.

### References

- [Source: bmad-output/EPIC-06-PLUGIN-SYSTEM.md#Feature-61-Plugin-Manifest-Format]
- [Source: bmad-output/EPIC-06-PLUGIN-SYSTEM.md#US-611-Manifest-Schema-Definition]
- [Source: bmad-output/EPIC-06-PLUGIN-SYSTEM.md#US-612-Manifest-Validation]

## Change Log

- 2026-08-03: Initial story creation
- 2026-08-03: Implemented manifest types + validation in `types/src/plugin.rs`; 8 unit tests pass.

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free

### Debug Log References

### Completion Notes List

- Created `types/src/plugin.rs` with `PluginManifest` (schema_version, name, version, author, entry_point required; description/homepage/license/permissions/ui_components/dependencies/config_schema optional), `ValidationIssue`, `ValidationReport`.
- `validate_manifest(json) -> ValidationReport` returns manifest on success (warnings surfaced via report.warnings); errors carry field path + reason.
- Unknown permissions warn (forward-compatible), non-strict-semver versions warn + best-effort parse; `parse_semver_best_effort` + `is_strict_semver` helpers.
- Recognized permission kinds: dbus, http, https, unix_socket, system, api, notification.
- Note: manifest types live in the existing `types` crate (`dyd-types`) as a `plugin` module rather than a new `dyd-plugin-types` crate, matching the established workspace pattern (story notes allowed either).
- 8 unit tests cover: valid parse, missing required fields, unknown permission warning, non-semver warning, invalid JSON, config_schema passthrough, wrong field type, semver parser edges. All pass.

### File List

- types/src/plugin.rs (new)
- types/src/lib.rs (modified: pub mod plugin;)