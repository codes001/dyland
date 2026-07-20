import { create } from "zustand";
import type { NotificationPayload } from "../../../types/src/index";

export interface NotificationStore {
  notification: NotificationPayload | null;
  setNotification: (payload: NotificationPayload) => void;
  clearNotification: () => void;
}

export const useNotificationStore = create<NotificationStore>((set) => ({
  notification: null,

  setNotification: (payload: NotificationPayload) => {
    set({ notification: payload });
  },

  clearNotification: () => {
    set({ notification: null });
  },
}));
