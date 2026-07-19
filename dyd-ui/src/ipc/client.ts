import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { usePillStore } from "../stores/pill-store";
import { useActivityStore } from "../stores/activity-store";
import type { IpcEnvelope } from "@types/index";

type ConnectionCallback = (status: ConnectionStatus) => void;

export type ConnectionStatus = "connecting" | "connected" | "disconnected" | "reconnecting";

const RECONNECT_INTERVAL = 2000;
const MAX_RECONNECT_DELAY = 30000;

class IpcClient {
  private reconnectAttempt = 0;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private callbacks: ConnectionCallback[] = [];
  private status: ConnectionStatus = "disconnected";
  private unlistenMsg: UnlistenFn | null = null;
  private unlistenStatus: UnlistenFn | null = null;

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
      case "DndChanged": {
        break;
      }
      case "ConfigChanged": {
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
