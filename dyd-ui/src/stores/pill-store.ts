import { create } from "zustand";
import type { PillState } from "../types/pill";
import { STATE_RULES, TRANSITIONS_TO_HIDDEN, type TransitionConfig } from "../types/pill";
import { IDLE_TIMEOUT } from "../config/animation";

export interface PillStore {
  state: PillState;
  previousState: PillState | null;
  transitionConfig: TransitionConfig | null;
  isFullscreen: boolean;
  idleTimer: ReturnType<typeof setTimeout> | null;

  transitionTo: (target: PillState) => void;
  handleFullscreenChange: (fullscreen: boolean) => void;
  resetIdleTimer: () => void;
  cleanup: () => void;
}

export const usePillStore = create<PillStore>((set, get) => ({
  state: "dormant",
  previousState: null,
  transitionConfig: null,
  isFullscreen: false,
  idleTimer: null,

  transitionTo: (target: PillState) => {
    const { state, idleTimer } = get();
    if (!STATE_RULES[state]?.includes(target)) return;
    if (state === target) return;

    const fromTransitions = TRANSITIONS_TO_HIDDEN;
    const config = target === "hidden"
      ? fromTransitions[state]
      : undefined;

    if (idleTimer) clearTimeout(idleTimer);

    set({
      previousState: state,
      state: target,
      transitionConfig: config ?? null,
      idleTimer: null,
    });

    if (target === "active") {
      const timer = setTimeout(() => {
        const current = get();
        if (current.state === "active" && !current.isFullscreen) {
          current.transitionTo("dormant");
        }
      }, IDLE_TIMEOUT);
      set({ idleTimer: timer });
    }
  },

  handleFullscreenChange: (fullscreen: boolean) => {
    const { state, transitionTo, previousState } = get();
    set({ isFullscreen: fullscreen });
    if (fullscreen && state !== "hidden") {
      transitionTo("hidden");
    } else if (!fullscreen && state === "hidden") {
      transitionTo(previousState ?? "dormant");
    }
  },

  resetIdleTimer: () => {
    const { state, idleTimer } = get();
    if (state === "active") {
      if (idleTimer) clearTimeout(idleTimer);
      const timer = setTimeout(() => {
        const current = get();
        if (current.state === "active" && !current.isFullscreen) {
          current.transitionTo("dormant");
        }
      }, IDLE_TIMEOUT);
      set({ idleTimer: timer });
    }
  },

  cleanup: () => {
    const { idleTimer } = get();
    if (idleTimer) clearTimeout(idleTimer);
  },
}));
