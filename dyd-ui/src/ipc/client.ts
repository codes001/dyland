import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { usePillStore } from "../stores/pill-store";
import { useActivityStore } from "../stores/activity-store";
import { useMediaStore } from "../stores/media-store";
import { useNotificationStore } from "../stores/notification-store";
import { useSystemStore } from "../stores/system-store";
import type {
  BatteryPayload,
  BluetoothPayload,
  BrightnessPayload,
  IpcEnvelope,
  NotificationPayload,
  VolumePayload,
  WifiPayload,
} from "../../../types/src/index";

type ConnectionCallback = (status: ConnectionStatus) => void;

export type ConnectionStatus = "connecting" | "connected" | "disconnected" | "reconnecting";

const RECONNECT_INTERVAL = 2000;
const MAX_RECONNECT_DELAY = 30000;
const NOTIFICATION_DISMISS_MS = 5000;

class IpcClient {
  private reconnectAttempt = 0;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private callbacks: ConnectionCallback[] = [];
  private status: ConnectionStatus = "disconnected";
  private unlistenMsg: UnlistenFn | null = null;
  private unlistenStatus: UnlistenFn | null = null;
  private notificationTimer: ReturnType<typeof setTimeout> | null = null;

  async connect() {
    if (this.status === "connected" || this.status === "connecting") return;
    this.setStatus("connecting");
    await this.doConnect();
  }

  private async doConnect() {
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

      const result = await invoke<string>("connect_daemon");
      if (result === "connecting") {
        this.setStatus("connecting");
      }
    } catch {
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
      case "NotificationReceived": {
        const p = envelope.payload as NotificationPayload;
        if (p?.app_name) {
          useNotificationStore.getState().setNotification(p);
          if (this.notificationTimer) clearTimeout(this.notificationTimer);
          this.notificationTimer = setTimeout(() => {
            useNotificationStore.getState().clearNotification();
          }, NOTIFICATION_DISMISS_MS);
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
          useSystemStore.getState().setVolume(p);
        }
        break;
      }
      case "BrightnessChanged": {
        const p = envelope.payload as BrightnessPayload;
        if (p && typeof p.has_backlight === "boolean") {
          useSystemStore.getState().setBrightness(p);
        }
        break;
      }
      case "DndChanged": {
        break;
      }
      case "ConfigChanged": {
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
