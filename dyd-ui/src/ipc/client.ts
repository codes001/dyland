import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { usePillStore } from "../stores/pill-store";
import { useActivityStore } from "../stores/activity-store";
import { useMediaStore } from "../stores/media-store";
import { useNotificationStore } from "../stores/notification-store";
import { useSystemStore } from "../stores/system-store";
import { usePluginStore } from "../stores/plugin-store";
import { useConfigStore } from "../stores/config-store";
import { useTransientStore } from "../stores/transient-store";
import type {
  AppConfig,
  BatteryPayload,
  BluetoothPayload,
  BrightnessPayload,
  IpcEnvelope,
  NotificationPayload,
  PluginHealthPayload,
  PluginListPayload,
  VolumePayload,
  WifiPayload,
} from "../../../types/src/index";

type ConnectionCallback = (status: ConnectionStatus) => void;

export type ConnectionStatus = "connecting" | "connected" | "disconnected" | "reconnecting";

const RECONNECT_INTERVAL = 2000;
const MAX_RECONNECT_DELAY = 30000;
const NOTIFICATION_DISMISS_MS = 5000;
const ACTIVITY_DISMISS_MS = 6000;

class IpcClient {
  private reconnectAttempt = 0;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private callbacks: ConnectionCallback[] = [];
  private status: ConnectionStatus = "disconnected";
  private unlistenMsg: UnlistenFn | null = null;
  private unlistenStatus: UnlistenFn | null = null;
  private notificationTimer: ReturnType<typeof setTimeout> | null = null;
  private generation = 0;

  async connect() {
    if (this.status === "connected" || this.status === "connecting" || this.status === "reconnecting") return;
    this.setStatus("connecting");
    await this.doConnect();
  }

  private async doConnect() {
    const gen = ++this.generation;
    if (this.unlistenMsg) {
      this.unlistenMsg();
      this.unlistenMsg = null;
    }
    if (this.unlistenStatus) {
      this.unlistenStatus();
      this.unlistenStatus = null;
    }
    const isTauriEnv = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
    if (!isTauriEnv) {
      // Standard browser preview mode: inject mock system state so the UI is fully interactive in browser
      this.setStatus("connected");
      this.handleDaemonMessage({
        type: "BatteryChanged",
        payload: { state: "discharging", percentage: 0.0, is_present: false, time_to_empty: null, time_to_full: null, health: null },
      });
      this.handleDaemonMessage({
        type: "WifiChanged",
        payload: { state: "connected", ssid: "Home-WiFi", signal_dbm: -52, signal_percent: 88, frequency_mhz: 5240, ip_v4: "192.168.1.105" },
      });
      this.handleDaemonMessage({
        type: "BluetoothChanged",
        payload: { powered: true, discoverable: false, present: true, devices: [{ name: "Headphones", connected: true, icon: null, address: "aa:bb:cc:dd:ee:ff" }] },
      });
      this.handleDaemonMessage({
        type: "VolumeChanged",
        payload: { volume: 0.65, muted: false, sink_name: "Speaker" },
      });
      this.handleDaemonMessage({
        type: "BrightnessChanged",
        payload: { brightness: 80, has_backlight: true },
      });
      return;
    }

    try {
      this.unlistenMsg = await listen<unknown>("daemon-message", (event) => {
        this.handleDaemonMessage(event.payload);
      });
      this.unlistenStatus = await listen<{ status: string }>("daemon-status", (event) => {
        if (event.payload.status === "connected") {
          this.reconnectAttempt = 0;
          this.setStatus("connected");
        } else {
          this.scheduleReconnect();
        }
      });
      if (gen !== this.generation) return;

      const result = await invoke<string>("connect_daemon");
      if (gen !== this.generation) return;
      if (result === "connecting") {
        this.setStatus("connecting");
      }
    } catch {
      if (gen !== this.generation) return;
      this.scheduleReconnect();
    }
  }

  private handleDaemonMessage(payload: unknown) {
    const envelope = payload as IpcEnvelope;
    if (!envelope?.type) return;

    switch (envelope.type) {
      case "FullscreenChanged": {
        const p = envelope.payload as { is_fullscreen: boolean; monitor?: string };
        if (p && typeof p.is_fullscreen === "boolean") {
          usePillStore.getState().handleFullscreenChange(p.is_fullscreen);
        }
        break;
      }
      case "ActivityQueued": {
        const p = envelope.payload as { activity_id: string; source: string; content: unknown };
        if (p?.activity_id) {
          useActivityStore.getState().push({
            id: p.activity_id,
            source: p.source,
            content: p.content,
            priority: "medium",
          });
        }
        break;
      }
      case "ActivityDequeued": {
        const p = envelope.payload as { activity_id: string };
        if (p?.activity_id) {
          useActivityStore.getState().remove(p.activity_id);
        }
        break;
      }
      case "QueueEmptied": {
        useActivityStore.getState().clear();
        break;
      }
      case "PluginActivity": {
        const p = envelope.payload as {
          activity_id: string;
          source: string;
          content: unknown;
        };
        if (p?.activity_id) {
          const content = (p.content ?? {}) as { severity?: string };
          const priority =
            content.severity === "critical"
              ? "critical"
              : content.severity === "error"
                ? "high"
                : "medium";
          useActivityStore.getState().push({
            id: p.activity_id,
            source: p.source,
            content: p.content,
            priority,
          });
          const { state, isFullscreen } = usePillStore.getState();
          if (!isFullscreen && (state === "dormant" || state === "hidden")) {
            usePillStore.getState().transitionTo("active");
          }
          setTimeout(() => {
            useActivityStore.getState().remove(p.activity_id);
          }, ACTIVITY_DISMISS_MS);
        }
        break;
      }
      case "NotificationReceived": {
        const p = envelope.payload as NotificationPayload;
        if (p?.app_name) {
          useNotificationStore.getState().setNotification(p);
          if (this.notificationTimer) clearTimeout(this.notificationTimer);
          this.notificationTimer = setTimeout(() => {
            if (usePillStore.getState().state === "expanded") {
              this.notificationTimer = setTimeout(() => {
                useNotificationStore.getState().clearNotification();
              }, NOTIFICATION_DISMISS_MS);
              return;
            }
            useNotificationStore.getState().clearNotification();
          }, NOTIFICATION_DISMISS_MS);
          const { state, isFullscreen } = usePillStore.getState();
          if (!isFullscreen && (state === "dormant" || state === "hidden")) {
            usePillStore.getState().transitionTo("active");
          }
        }
        break;
      }
      case "BatteryChanged": {
        const p = envelope.payload as BatteryPayload;
        if (p && typeof p.is_present === "boolean") {
          useSystemStore.getState().setBattery(p);
        }
        break;
      }
      case "WifiChanged": {
        const p = envelope.payload as WifiPayload;
        if (p && typeof p.state === "string") {
          useSystemStore.getState().setWifi(p);
        }
        break;
      }
      case "BluetoothChanged": {
        const p = envelope.payload as BluetoothPayload;
        if (p && typeof p.powered === "boolean") {
          useSystemStore.getState().setBluetooth(p);
        }
        break;
      }
      case "VolumeChanged": {
        const p = envelope.payload as VolumePayload;
        if (p && typeof p.volume === "number") {
          const prev = useSystemStore.getState().volume;
          useSystemStore.getState().setVolume(p);
          if (prev && prev.volume !== p.volume) {
            useTransientStore.getState().pulse("volume");
          }
        }
        break;
      }
      case "BrightnessChanged": {
        const p = envelope.payload as BrightnessPayload;
        if (p && typeof p.has_backlight === "boolean") {
          const prev = useSystemStore.getState().brightness;
          useSystemStore.getState().setBrightness(p);
          if (prev && prev.brightness !== p.brightness) {
            useTransientStore.getState().pulse("brightness");
          }
        }
        break;
      }
      case "DndChanged": {
        break;
      }
      case "PluginList": {
        const p = envelope.payload as PluginListPayload;
        if (p && Array.isArray(p.plugins)) {
          usePluginStore.getState().setPlugins(p.plugins);
        }
        break;
      }
      case "PluginChanged": {
        const p = envelope.payload as PluginHealthPayload;
        if (p?.name) {
          usePluginStore.getState().updatePlugin(p);
        }
        break;
      }
      case "ConfigChanged": {
        const p = envelope.payload as { changes?: AppConfig } | AppConfig;
        const config = (p && "changes" in p ? p.changes : p) as AppConfig;
        if (config && typeof config === "object") {
          useConfigStore.getState().setConfig(config);
        }
        break;
      }
      case "MediaChanged": {
        const p = envelope.payload as {
          player: string;
          title: string;
          artist: string;
          album: string;
          art_url: string | null;
          length: number;
          position: number;
          playback_status: string;
        };
        if (p?.playback_status === "stopped" || (!p?.title && !p?.artist) || !p?.player) {
          useMediaStore.getState().clearMedia();
        } else {
          useMediaStore.getState().setMedia({
            player: p.player,
            title: p.title,
            artist: p.artist,
            album: p.album,
            artUrl: p.art_url,
            length: p.length,
            position: p.position,
            playbackStatus: p.playback_status,
          });
        }
        break;
      }
    }
  }

  async sendCommand(command: string): Promise<void> {
    await invoke("send_command", { command });
  }

  // --- plugin management commands (story 6-5) -------------------------------
  // Each serializes a typed `IpcCommand` and reuses the `send_command` bridge.

  private async sendPluginCommand(cmd: Record<string, unknown>): Promise<void> {
    await this.sendCommand(JSON.stringify(cmd));
  }

  /** Request the current plugin list (daemon replies with a `PluginList` event). */
  async getPlugins(): Promise<void> {
    await this.sendPluginCommand({ type: "GetPlugins" });
  }

  /** Request a single plugin's health (daemon replies with `PluginChanged`). */
  async getPluginHealth(name: string): Promise<void> {
    await this.sendPluginCommand({ type: "GetPluginHealth", payload: { name } });
  }

  /** Enable or disable a plugin. Disabling stops it without unloading. */
  async setPluginEnabled(name: string, enabled: boolean): Promise<void> {
    await this.sendPluginCommand({
      type: "SetPluginEnabled",
      payload: { name, enabled },
    });
  }

  /** Uninstall a plugin (daemon stops it first, then removes it). */
  async uninstallPlugin(name: string): Promise<void> {
    await this.sendPluginCommand({ type: "UninstallPlugin", payload: { name } });
  }

  /** Request current application config from daemon. */
  async getConfig(): Promise<void> {
    await this.sendPluginCommand({ type: "GetConfig" });
  }

  /** Update application config in daemon. */
  async updateConfig(changes: Record<string, unknown>): Promise<void> {
    await this.sendPluginCommand({ type: "UpdateConfig", payload: { changes } });
  }

  /** Push a new config object for a plugin. */
  async setPluginConfig(name: string, config: unknown): Promise<void> {
    await this.sendPluginCommand({
      type: "SetPluginConfig",
      payload: { name, config },
    });
  }

  private scheduleReconnect() {
    if (this.reconnectTimer) return;
    this.setStatus("reconnecting");
    const delay = Math.min(
      RECONNECT_INTERVAL * Math.pow(2, this.reconnectAttempt),
      MAX_RECONNECT_DELAY,
    );
    this.reconnectAttempt++;
    this.reconnectTimer = setTimeout(async () => {
      this.reconnectTimer = null;
      await this.doConnect();
    }, delay);
  }

  private setStatus(status: ConnectionStatus) {
    if (this.status === status) return;
    this.status = status;
    this.callbacks.forEach((cb) => cb(status));
  }

  onStatusChange(callback: ConnectionCallback) {
    this.callbacks.push(callback);
    callback(this.status);
    return () => {
      this.callbacks = this.callbacks.filter((cb) => cb !== callback);
    };
  }

  getStatus(): ConnectionStatus {
    return this.status;
  }

  disconnect() {
    this.generation++;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    if (this.unlistenMsg) {
      this.unlistenMsg();
      this.unlistenMsg = null;
    }
    if (this.unlistenStatus) {
      this.unlistenStatus();
      this.unlistenStatus = null;
    }
    this.setStatus("disconnected");
  }
}

export const ipcClient = new IpcClient();
