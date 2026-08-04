# Deferred Work

Deferred items recorded during code review (Epic 5). Each entry references the originating story file where the finding also lives.

## Epic 5 — System Status Indicators

- [ ] 5-2: Connecting indicator is static (no spinner) — 5-2 AC7 vs 5-6 AC10 static-icons; resolved in favor of 5-6. [dyd-ui/src/components/status/WifiIndicator.tsx]
- [ ] 5-4: sink_name always None on wpctl path — sanctioned by 5-4 Dev Notes. [dyd-daemon/src/sources/volume.rs]
- [ ] 5-4: 200ms poll subprocess churn — spec-mandated polling. [dyd-daemon/src/sources/volume.rs]
- [ ] 5-4: NaN/Inf coerced to 0 in SetVolume/SetMute clamp — matches existing media-source clamp pattern. [dyd-daemon/src/ipc/connection.rs]
- [ ] 5-1: Health can show "Health 0%" when EnergyFull=0 — acceptable edge, not a hard violation. [dyd-daemon/src/sources/battery.rs]
- [ ] 5-3: Bluetooth downcast_ref "compile errors" flagged by Blind Hunter — disproven by clean build; retained for record. [dyd-daemon/src/sources/bluetooth.rs]
- [ ] 5-6: Late-connecting clients never receive initial source state — sources only broadcast on change; needs a state cache + replay or resync-on-connect design. Decision: defer (state converges on next event). [dyd-daemon/src/ipc/connection.rs]

## Deferred from: code review of 4-2-notification-rendering (2026-08-03)

- [ ] Empty app_name notifications dropped by daemon-message gate — pre-existing in client.ts from story 4-1. [dyd-ui/src/ipc/client.ts:101]
- [ ] Dismiss sends NotificationClose with daemon's replaces_id (often 0) — server-side no-op for first notifications. Pre-existing daemon behavior. [dyd-daemon/src/ipc/connection.rs]
- [ ] Icon resolution implements only the path branch — full US-4.2.3 chain (image-data, image-path, desktop-file/icon-theme lookup) out of scope; story AC12 only requires app_icon path -> cached -> bell. [dyd-ui/src/lib/notification-utils.ts]
