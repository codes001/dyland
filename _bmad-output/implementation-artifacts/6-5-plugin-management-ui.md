# Story 6.5: Plugin Management UI

Status: ready-for-dev

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

- [ ] Add plugin types to UI (AC: 1, 7)
  - [ ] Add `PluginInfo`, `PluginHealth` interfaces in a new `dyd-ui/src/types/plugin.ts`
  - [ ] Map status strings to badge colors (running=green, error=yellow, disabled=gray)
- [ ] Add plugin store (AC: 1, 3, 4)
  - [ ] Create `dyd-ui/src/stores/plugin-store.ts` (Zustand)
  - [ ] State: `plugins: PluginInfo[]`, `selectedPlugin`, actions `setPlugins`, `setSelected`, `updatePlugin`
  - [ ] Real-time updates from daemon `PluginChanged` events
- [ ] Handle plugin IPC in client (AC: 1, 5, 6)
  - [ ] Handle `PluginChanged` event in `dyd-ui/src/ipc/client.ts`
  - [ ] Add send commands for `GetPlugins`, `SetPluginEnabled`, `UninstallPlugin`, `GetPluginHealth`, `SetPluginConfig`
- [ ] Create PluginList component (AC: 1, 2, 4)
  - [ ] Create `dyd-ui/src/components/settings/PluginList.tsx` — list rows with name, version, status badge
  - [ ] Click row → detail view
  - [ ] Empty state text + plugin count summary
- [ ] Create PluginDetail component (AC: 2, 5, 6, 7)
  - [ ] Create `dyd-ui/src/components/settings/PluginDetail.tsx`
  - [ ] Enable/disable toggle
  - [ ] Uninstall button with confirmation dialog; "This is a system plugin" warning for core
  - [ ] Error-state actions: Disable + Reset buttons
- [ ] Create PluginConfigForm component (AC: 8, 9, 10, 11)
  - [ ] Create `dyd-ui/src/components/settings/PluginConfigForm.tsx`
  - [ ] Render form fields from `config_schema`: string/number/boolean/enum/array/object
  - [ ] Show field label + description; readOnly fields non-editable
  - [ ] 1s debounced auto-save via `SetPluginConfig`; "Reset to defaults" button; required-field validation
- [ ] Wire settings panel (AC: 1)
  - [ ] Integrate plugin list/detail into an existing settings area or a new Settings view; note if settings UI does not yet exist in the app
- Add tests (AC: 1, 8)
  - [ ] Unit test status → badge color mapping
  - [ ] Unit test config-schema → field descriptor generation (string/number/boolean/enum/array/object)
  - [ ] Test debounce helper logic if isolated

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

## Dev Agent Record

### Agent Model Used

opencode/deepseek-v4-flash-free

### Debug Log References

### Completion Notes List

### File List