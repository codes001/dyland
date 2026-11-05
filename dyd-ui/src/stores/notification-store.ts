import { create } from "zustand";
import type { NotificationPayload } from "../../../types/src/index";

export interface IslandCall extends NotificationPayload {
  phase?: string;
  started_at?: number;
}

export interface NotificationStore {
  notification: NotificationPayload | null;
  call: IslandCall | null;
  setNotification: (payload: NotificationPayload) => void;
  clearNotification: () => void;
  setCall: (payload: IslandCall) => void;
  clearCall: () => void;
}

export const useNotificationStore = create<NotificationStore>((set) => ({
  notification: null,
  call: null,

  setNotification: (payload: NotificationPayload) => {
    set({ notification: payload });
  },

  clearNotification: () => {
    set({ notification: null });
  },

  setCall: (payload: NotificationPayload) => {
    set({ call: { ...payload, urgency: "call" } });
  },

  clearCall: () => {
    set({ call: null });
  },
}));
