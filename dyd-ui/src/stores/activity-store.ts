import { create } from "zustand";

export interface Activity {
  id: string;
  source: string;
  content: unknown;
  priority: "critical" | "high" | "medium" | "low" | "info";
}

export interface ActivityStore {
  queue: Activity[];
  currentActivity: Activity | null;

  push: (activity: Activity) => void;
  remove: (id: string) => void;
  cycleNext: () => void;
  cyclePrev: () => void;
  clear: () => void;
  setCurrent: (activity: Activity | null) => void;
}

export const useActivityStore = create<ActivityStore>((set, get) => ({
  queue: [],
  currentActivity: null,

  push: (activity: Activity) => {
    const { queue } = get();
    if (queue.some((a) => a.id === activity.id)) return;

    const priorityRank = { critical: 5, high: 4, medium: 3, low: 2, info: 1 };
    const sorted = [...queue, activity].sort(
      (a, b) => (priorityRank[b.priority] ?? 0) - (priorityRank[a.priority] ?? 0),
    );

    const MAX = 20;
    const trimmed = sorted.length > MAX ? sorted.slice(0, MAX) : sorted;

    set({
      queue: trimmed,
      currentActivity: get().currentActivity ?? trimmed[0] ?? null,
    });
  },

  remove: (id: string) => {
    const { queue, currentActivity } = get();
    const filtered = queue.filter((a) => a.id !== id);
    set({ queue: filtered });
    if (currentActivity?.id === id) {
      set({ currentActivity: filtered[0] ?? null });
    }
  },

  cycleNext: () => {
    const { queue, currentActivity } = get();
    if (queue.length === 0 || !currentActivity) return;
    const idx = queue.findIndex((a) => a.id === currentActivity.id);
    const next = queue[(idx + 1) % queue.length];
    set({ currentActivity: next });
  },

  cyclePrev: () => {
    const { queue, currentActivity } = get();
    if (queue.length === 0 || !currentActivity) return;
    const idx = queue.findIndex((a) => a.id === currentActivity.id);
    const prev = queue[(idx - 1 + queue.length) % queue.length];
    set({ currentActivity: prev });
  },

  clear: () => {
    set({ queue: [], currentActivity: null });
  },

  setCurrent: (activity: Activity | null) => {
    set({ currentActivity: activity });
  },
}));
