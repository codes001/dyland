export type IpcEvent =
  | { type: "Pong"; payload: IpcPongPayload }
  | { type: "MediaChanged"; payload: MediaChangedPayload }
  | { type: "NotificationReceived"; payload: NotificationPayload }
  | { type: "BatteryChanged"; payload: BatteryPayload }
  | { type: "WifiChanged"; payload: WifiPayload }
  | { type: "BluetoothChanged"; payload: BluetoothPayload }
  | { type: "VolumeChanged"; payload: VolumePayload }
  | { type: "BrightnessChanged"; payload: BrightnessPayload }
  | { type: "ConnectionStatus"; payload: ConnectionStatusPayload }
  | { type: "ActivityQueued"; payload: ActivityPayload }
  | { type: "ActivityDequeued"; payload: ActivityPayload }
  | { type: "ActivityChanged"; payload: ActivityPayload }
  | { type: "QueueEmptied" }
  | { type: "FullscreenChanged"; payload: FullscreenPayload }
  | { type: "DndChanged"; payload: DndPayload }
  | { type: "ConfigChanged"; payload: ConfigChangedPayload }
  | { type: "PluginChanged"; payload: PluginHealthPayload }
  | { type: "PluginActivity"; payload: ActivityPayload }
  | { type: "PluginList"; payload: PluginListPayload };

export type IpcCommand =
  | { type: "Ping" }
  | { type: "GetState" }
  | { type: "DismissActivity"; payload: { activity_id: string } }
  | { type: "CycleNext" }
  | { type: "CyclePrev" }
  | { type: "MediaPlayPause"; payload: { player: string } }
  | { type: "MediaNext"; payload: { player: string } }
  | { type: "MediaPrevious"; payload: { player: string } }
  | { type: "MediaSeek"; payload: { player: string; position_us: number } }
  | { type: "MediaSetVolume"; payload: { player: string; volume: number } }
  | { type: "CyclePlayer"; payload: { player?: string } }
  | { type: "NotificationAction"; payload: { id: number; action_key: string; reply_text?: string } }
  | { type: "NotificationClose"; payload: { id: number } }
  | { type: "SetDnd"; payload: { enabled: boolean } }
  | { type: "SetVolume"; payload: { volume: number } }
  | { type: "SetMute"; payload: { muted: boolean } }
  | { type: "SetBrightness"; payload: { brightness: number } }
  | { type: "SetBluetoothPowered"; payload: { powered: boolean } }
  | { type: "GetConfig" }
  | { type: "UpdateConfig"; payload: { changes: unknown } }
  | { type: "GetPlugins" }
  | { type: "GetPluginHealth"; payload: { name: string } }
  | { type: "InstallPlugin"; payload: { path: string } }
  | { type: "SetPluginEnabled"; payload: { name: string; enabled: boolean } }
  | { type: "UninstallPlugin"; payload: { name: string } }
  | { type: "SetPluginConfig"; payload: { name: string; config: unknown } };

export interface IpcEnvelope {
  id: string;
  timestamp: number;
  kind: "event" | "command";
  type: string;
  payload?: unknown;
}

export interface IpcPongPayload {
  status: string;
  uptime_secs: number;
  version: string;
  connected_clients: number;
}

export interface MediaChangedPayload {
  player: string;
  title: string;
  artist: string;
  album: string;
  art_url: string | null;
  length: number;
  position: number;
  playback_status: string;
}

export interface NotificationActionItem {
  key: string;
  label: string;
}

export interface NotificationPayload {
  id: number;
  app_name: string;
  app_icon: string | null;
  summary: string;
  body: string | null;
  urgency: string;
  actions: NotificationActionItem[];
  timestamp: number;
}

export interface BatteryPayload {
  percentage: number;
  state: string;
  time_to_empty: number | null;
  time_to_full: number | null;
  is_present: boolean;
  health: number | null;
}

export interface WifiPayload {
  ssid: string | null;
  strength: number;
  state: string;
  ip_address: string | null;
  frequency_mhz: number | null;
  security: string | null;
}

export interface BluetoothDevice {
  name: string;
  connected: boolean;
  icon: string | null;
  address: string;
}

export interface BluetoothPayload {
  powered: boolean;
  discoverable: boolean;
  present: boolean;
  devices: BluetoothDevice[];
}

export interface VolumePayload {
  volume: number;
  muted: boolean;
  sink_name: string | null;
}

export interface BrightnessPayload {
  brightness: number;
  has_backlight: boolean;
}

export interface ConnectionStatusPayload {
  status: string;
}

export interface ActivityPayload {
  activity_id: string;
  source: string;
  content: unknown;
}

export interface FullscreenPayload {
  is_fullscreen: boolean;
  monitor: string | null;
}

export interface DndPayload {
  enabled: boolean;
}

export interface ConfigChangedPayload {
  changes: unknown;
}

/** Root application configuration (Epic 7). Mirror of Rust `AppConfig`.
 * Every field is optional here because the daemon always sends a fully
 * hydrated object, but partial `UpdateConfig` patches send only changed keys. */
export interface AppConfig {
  general: GeneralConfig;
  appearance: AppearanceConfig;
  accessibility: AccessibilityConfig;
  features: FeatureToggles;
  notifications: NotificationConfig;
  shortcuts: ShortcutConfig;
  last_category: string | null;
}

export interface GeneralConfig {
  start_on_login: boolean;
}

export type ThemePreference = "dark" | "light" | "system";

export interface AppearanceConfig {
  theme: ThemePreference;
}

export interface AccessibilityConfig {
  /** null = follow system `prefers-reduced-motion`. */
  reduce_motion: boolean | null;
  /** null = follow system `prefers-contrast`. */
  high_contrast: boolean | null;
  /** null = 1.0 (default font scale, ranges 0.75 to 1.5). */
  font_scale: number | null;
}

export interface AppNotificationRule {
  allow: boolean;
  show_silently: boolean;
  queue_only: boolean;
}

export interface NotificationConfig {
  dnd_enabled: boolean;
  app_rules: Record<string, AppNotificationRule>;
}

export interface ShortcutConfig {
  bindings: Record<string, string>;
}

export interface FeatureToggles {
  battery: boolean;
  wifi: boolean;
  bluetooth: boolean;
  volume: boolean;
  brightness: boolean;
  media: boolean;
  notifications: boolean;
}

export interface PluginHealthPayload {
  name: string;
  version: string;
  state: string;
  last_error: string | null;
  error_count: number;
  uptime_secs: number;
  memory_usage: number | null;
  disabled: boolean;
  is_core: boolean;
  permissions: string[];
  ui_components: string[];
  config_schema: unknown | null;
  config: unknown | null;
}

export interface PluginListPayload {
  plugins: PluginHealthPayload[];
}
