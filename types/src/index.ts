export type IpcEvent =
  | { type: "Pong"; payload: IpcPongPayload }
  | { type: "MediaChanged"; payload: MediaChangedPayload }
  | { type: "NotificationReceived"; payload: NotificationPayload }
  | { type: "BatteryChanged"; payload: BatteryPayload }
  | { type: "ConnectionStatus"; payload: ConnectionStatusPayload }
  | { type: "ActivityQueued"; payload: ActivityPayload }
  | { type: "ActivityDequeued"; payload: ActivityPayload }
  | { type: "QueueEmptied" }
  | { type: "ConfigChanged"; payload: ConfigChangedPayload };

export type IpcCommand =
  | { type: "Ping" }
  | { type: "GetState" }
  | { type: "DismissActivity"; payload: { activity_id: string } }
  | { type: "CycleNext" }
  | { type: "CyclePrev" }
  | { type: "SetDnd"; payload: { enabled: boolean } }
  | { type: "GetConfig" };

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

export interface NotificationPayload {
  id: number;
  app_name: string;
  app_icon: string | null;
  summary: string;
  body: string | null;
  urgency: string;
  timestamp: number;
}

export interface BatteryPayload {
  percentage: number;
  state: string;
  time_to_empty: number | null;
  time_to_full: number | null;
  is_present: boolean;
}

export interface ConnectionStatusPayload {
  status: string;
}

export interface ActivityPayload {
  activity_id: string;
  source: string;
  content: unknown;
}

export interface ConfigChangedPayload {
  changes: unknown;
}
