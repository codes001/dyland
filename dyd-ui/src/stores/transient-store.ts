import { create } from "zustand";

export type TransientKind = "volume" | "brightness";

export const TRANSIENT_ACTIVITY_MS = 4000;

interface TransientStore {
  active: Record<TransientKind, boolean>;
  pulse: (kind: TransientKind) => void;
}

const timers: Record<TransientKind, ReturnType<typeof setTimeout> | null> = {
  volume: null,
  brightness: null,
};

export const useTransientStore = create<TransientStore>((set, get) => ({
  active: { volume: false, brightness: false },

  pulse: (kind) => {
    if (timers[kind]) clearTimeout(timers[kind]);
    set({ active: { ...get().active, [kind]: true } });
    timers[kind] = setTimeout(() => {
      timers[kind] = null;
      set({ active: { ...get().active, [kind]: false } });
    }, TRANSIENT_ACTIVITY_MS);
  },
}));
