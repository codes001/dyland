# Story 6.5: Plugin Management UI

Status: review

## Story

As an end user,
I want to manage installed plugins from the settings UI,
so that I can see what plugins are active, enable/disable/uninstall them, and configure them.

## Acceptance Criteria

1. Settings shows a list of installed plugins with name, version, and status badge (green=running, yellow=error, gray=disabled) (US-6.5.1)
2. Clicking a plugin opens a detail view (US-6.5.1)
3. Status updates in real-time (US-6.5.1)
4. Empty state: "No plugins installed. Browse the plugin marketplace." and a plugin count summary (US-6.5.1)
5. Toggle switch to enable/disable a plugin; disabled plugin → Stopped state (not unloaded) (US-6.5.2)
6. Uninstall button with confirmation dialog; stops a running plugin first, then uninstalls; core/bundled plugins show a warning (US-6.5.2)
7. Plugin in Error state shows "Disable" and "Reset" buttons (reset re-enables) (US-6.5.2)
8. Configuration UI auto-generated from the plugin's `config_schema` (JSON Schema) (US-6.5.3)
9. Supported form fields: string, number, boolean, enum (dropdown), array (tag input), object (nested) (US-6.5.3)
10. Each field shows label and description from schema; changes saved on apply or change (auto-save with 1s debounce); "Reset to defaults" button (US-6.5.3)
11. Required fields validated before save; read-only fields (`readOnly:true`) displayed but not editable (US-6.5.3)

## Tasks / Subtasks

- [x] Add plugin types to UI (AC: 1, 7)
  - [x] Add `PluginInfo`, `PluginHealth` interfaces in a new `dyd-ui/src/types/plugin.ts`
  - [x] Map status strings to badge colors (running=green, error=yellow, disabled=gray)
- [x] Add plugin store (AC: 1, 3, 4)
  - [x] Create `dyd-ui/src/stores/plugin-store.ts` (Zustand)
  - [x] State: `plugins: PluginInfo[]`, `selectedPlugin`, actions `setPlugins`, `setSelected`, `updatePlugin`
  - [x] Real-time updates from daemon `PluginChanged` events
- [x] Handle plugin IPC in client (AC: 1, 5, 6)
  - [x] Handle `PluginChanged` event in `dyd-ui/src/ipc/client.ts`
  - [x] Add send commands for `GetPlugins`, `SetPluginEnabled`, `UninstallPlugin`, `GetPluginHealth`, `SetPluginConfig`
- [x] Create PluginList component (AC: 1, 2, 4)
  - [x] Create `dyd-ui/src/components/settings/PluginList.tsx` — list rows with name, version, status badge
  - [x] Click row → detail view
  - [x] Empty state text + plugin count summary
- [x] Create PluginDetail component (AC: 2, 5, 6, 7)
  - [x] Create `dyd-ui/src/components/settings/PluginDetail.tsx`
  - [x] Enable/disable toggle
  - [x] Uninstall button with confirmation dialog; "This is a system plugin" warning for core
  - [x] Error-state actions: Disable + Reset buttons
- [x] Create PluginConfigForm component (AC: 8, 9, 10, 11)
  - [x] Create `dyd-ui/src/components/settings/PluginConfigForm.tsx`
  - [x] Render form fields from `config_schema`: string/number/boolean/enum/array/object
  - [x] Show field label + description; readOnly fields non-editable
  - [x] 1s debounced auto-save via `SetPluginConfig`; "Reset to defaults" button; required-field validation
- [x] Wire settings panel (AC: 1)
  - [x] Integrate plugin list/detail into an existing settings area or a new Settings view; note if settings UI does not yet exist in the app
- Add tests (AC: 1, 8)
  - [x] Unit test status → badge color mapping
  - [x] Unit test config-schema → field descriptor generation (string/number/boolean/enum/array/object)
  - [x] Test debounce helper logic if isolated

## Dev Notes

- Feature 6.5 in `bmad-output/EPIC-06-PLUGIN-SYSTEM.md`. Plugin management is part of the Settings Epic (E7) but defined here (epic lines 433-435).
- The app currently has NO settings screen (exploration showed only media/status/notification components + stores). This story must introduce a minimal Settings container to host the plugin management components without building the full Epic-7 settings.
- IPC events/commands already defined in story 6-2 types; the client maps them (pattern: `dyd-ui/src/ipc/client.ts`).
- Availability/status comes from the daemon (PluginChanged / GetPlugins); config comes from manifest `config_schema` surfaced via GetPlugins detail.
- Follow existing Zustand store patterns (`d.yd-ui/src/stores/*`).
- If a full React JSON-schema form library is undesirable, a small custom renderer is acceptable (epic explicitly allows "custom form renderer", line 501).

### Project Structure Notes

- New: `dyd-ui/src/types/plugin.ts`
- New: `dyd-ui/src/stores/plugin-store.ts`
- New: `dyd-ui/src/components/settings/PluginList.tsx`
- New: `dyd-ui/src/components/settings/PluginDetail.tsx`
- New: `dyd-ui/src/components/settings/PluginConfigForm.tsx`
- Modify: `dyd-ui/src/ipc/client.ts`
- Modify: `dyd-ui/src/App.tsx` (add minimal Settings entry)

### References

- [Source: bmad-output/EPIC-06-PLUGIN-SYSTEM.md#Feature-65-Plugin-Management-UI]
- [Source: bmad-output/EPIC-06-PLUGIN-SYSTEM.md#US-651-Plugin-List-in-Settings]
- [Source: bmad-output/EPIC-06-PLUGIN-SYSTEM.md#US-652-Plugin-EnableDisableUninstall]
- [Source: bmad-output/EPIC-06-PLUGIN-SYSTEM.md#US-653-Plugin-Configuration-UI]
- [Source: existing dyd-ui/src/ipc/client.ts
- [Source: existing dyd-ui/src/stores/. for Zustand patterns]

## Change Log

- 2026-08-03: Initial story creation
- 2026-08-04: Story implemented and moved to review

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free

### Debug Log References

### Completion Notes List

1. **Minimal Settings container introduced**: The app had no settings surface. Added `Settings.tsx` — a lightweight container rendered inside the existing `ExpandedView` (opened via a new "Plugins" button in the no-activity expanded state). Full Epic-7 settings deferred, as the story allows.

2. **Custom form renderer over a schema library**: `PluginConfigForm` renders string/number/boolean/enum(dropdown)/array(comma-input)/object(nested) fields directly from `config_schema`, per the epic's explicit allowance for a custom renderer. All schema→field logic lives in the pure, unit-tested `lib/plugin-config.ts` (`schemaToFields`, `defaultsFor`, `validateRequired`, `debounce`).

3. **Testable pure logic split from React**: Presentation-free modules (`types/plugin.ts` badge map, `lib/plugin-config.ts`) hold all decision logic so they unit-test without a DOM. 15 tests across `tests/plugin-status.test.ts` and `tests/plugin-config.test.ts`, following the repo's existing `node:test` convention.

4. **IPC bridge reuse**: Plugin commands (`GetPlugins`/`GetPluginHealth`/`SetPluginEnabled`/`UninstallPlugin`/`SetPluginConfig`) serialize typed `IpcCommand`s through the existing `send_command` Tauri bridge — no new Rust command handlers needed. `PluginList`/`PluginChanged` events feed the Zustand store for real-time status (AC3).

5. **Error-state UX**: Errored plugins expose Disable + Reset (reset = re-enable) buttons and surface `last_error` + `error_count`. Uninstall requires an inline confirm; core (`is_core`) plugins show a "system plugin" warning.

6. **Auto-save**: Config edits auto-save after a 1s trailing debounce; required-field validation blocks the save until satisfied; `readOnly` fields render disabled. "Reset to defaults" cancels pending saves and pushes schema defaults.

### File List

**NEW:**
- `dyd-ui/src/types/plugin.ts` — PluginInfo/PluginHealth aliases, `badgeColor`/`statusLabel`/`BADGE_HEX` status→color map
- `dyd-ui/src/lib/plugin-config.ts` — `schemaToFields`, `defaultsFor`, `validateRequired`, `debounce` (pure logic)
- `dyd-ui/src/stores/plugin-store.ts` — Zustand store: plugins, selectedName, setPlugins/updatePlugin/removePlugin/setSelected
- `dyd-ui/src/components/settings/Settings.tsx` — minimal Settings container; requests plugins on mount
- `dyd-ui/src/components/settings/PluginList.tsx` — rows (name/version/status dot), empty state, count summary
- `dyd-ui/src/components/settings/PluginDetail.tsx` — enable/disable, error Reset, uninstall+confirm (core warning), config section
- `dyd-ui/src/components/settings/PluginConfigForm.tsx` — schema-driven form, debounced auto-save, validation, reset
- `dyd-ui/tests/plugin-status.test.ts` — 7 tests: badge color + status label mapping
- `dyd-ui/tests/plugin-config.test.ts` — 8 tests: schema→fields, defaults, required validation, debounce

**EDITED:**
- `dyd-ui/src/ipc/client.ts` — handle `PluginList`/`PluginChanged`; add 5 typed plugin send helpers
- `dyd-ui/src/App.tsx` — "Plugins" button + `Settings` overlay in the expanded view