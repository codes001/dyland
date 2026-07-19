export type PillState = "dormant" | "active" | "expanded" | "hidden";

export interface TransitionConfig {
  duration: number;
  easing: string;
  contentAnimation: "slide-up-fade" | "slide-down-fade" | "scale-expand" | "shrink-collapse" | "slide-up-off" | "slide-down-on";
}

export type TransitionMap = Record<string, Record<string, TransitionConfig>>;

export const TRANSITIONS: TransitionMap = {
  dormant: {
    active: { duration: 300, easing: "ease-out", contentAnimation: "slide-up-fade" },
  },
  active: {
    dormant: { duration: 200, easing: "ease-in", contentAnimation: "slide-down-fade" },
    expanded: { duration: 400, easing: "spring", contentAnimation: "scale-expand" },
  },
  expanded: {
    active: { duration: 300, easing: "spring", contentAnimation: "shrink-collapse" },
  },
  hidden: {
    active: { duration: 150, easing: "ease-out", contentAnimation: "slide-down-on" },
    dormant: { duration: 150, easing: "ease-out", contentAnimation: "slide-down-on" },
    expanded: { duration: 150, easing: "ease-out", contentAnimation: "slide-down-on" },
  },
};

export const TRANSITIONS_TO_HIDDEN: Record<string, TransitionConfig> = {
  dormant: { duration: 100, easing: "ease-out", contentAnimation: "slide-up-off" },
  active: { duration: 100, easing: "ease-out", contentAnimation: "slide-up-off" },
  expanded: { duration: 100, easing: "ease-out", contentAnimation: "slide-up-off" },
};

export const STATE_RULES: Record<PillState, string[]> = {
  dormant: ["active", "hidden"],
  active: ["dormant", "expanded", "hidden"],
  expanded: ["active", "hidden"],
  hidden: ["dormant", "active", "expanded"],
};
