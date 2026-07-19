import { ipcClient, type ConnectionStatus } from "../ipc/client";

export type { ConnectionStatus };

export type ConnectionListener = (status: ConnectionStatus) => void;

class ConnectionStore {
  private status: ConnectionStatus = "disconnected";
  private listeners: ConnectionListener[] = [];
  private unsubscribe: (() => void) | null = null;

  init() {
    if (this.unsubscribe) {
      this.unsubscribe();
    }
    this.unsubscribe = ipcClient.onStatusChange((status) => {
      this.status = status;
      this.listeners.forEach((cb) => cb(status));
    });
    ipcClient.connect();
  }

  subscribe(listener: ConnectionListener) {
    this.listeners.push(listener);
    listener(this.status);
    return () => {
      this.listeners = this.listeners.filter((cb) => cb !== listener);
    };
  }

  getStatus(): ConnectionStatus {
    return this.status;
  }

  cleanup() {
    if (this.unsubscribe) {
      this.unsubscribe();
      this.unsubscribe = null;
    }
    ipcClient.disconnect();
  }
}

export const connectionStore = new ConnectionStore();
