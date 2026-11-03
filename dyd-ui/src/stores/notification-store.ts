import { create } from "zustand";
import type { NotificationPayload } from "../../../types/src/index";

export interface NotificationStore {
  notification: NotificationPayload | null;
  call: NotificationPayload | null;
  setNotification: (payload: NotificationPayload) => void;
  clearNotification: () => void;
  setCall: (payload: NotificationPayload) => void;
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
