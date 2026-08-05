export const SPRING = {
  apple: { type: "spring" as const, stiffness: 380, damping: 28, mass: 0.8 },
  expand: { type: "spring" as const, stiffness: 400, damping: 28, mass: 0.8 },
  collapse: { type: "spring" as const, stiffness: 350, damping: 30, mass: 0.8 },
  quick: { type: "spring" as const, stiffness: 420, damping: 26, mass: 0.6 },
  hapticPulse: { type: "spring" as const, stiffness: 500, damping: 20, mass: 0.5 },
};

export const EASE = {
  out: [0.0, 0.0, 0.2, 1] as [number, number, number, number],
  in: [0.4, 0.0, 1, 1] as [number, number, number, number],
  inOut: [0.4, 0.0, 0.2, 1] as [number, number, number, number],
};

export const DURATION = {
  dormantToActive: 0.25,
  activeToDormant: 0.2,
  activeToExpanded: 0.35,
  expandedToActive: 0.25,
  toHidden: 0.1,
  hiddenToPrev: 0.15,
  contentEntry: 0.18,
  contentExit: 0.12,
  contentStagger: 0.04,
  hoverFade: 0.12,
  hoverExit: 0.15,
};

export const PILL_SIZE = {
  dormant: { width: 126, height: 36 },
  active: { width: 180, height: 36 },
  notification: { width: 200, height: 36 },
  expanded: { width: 380, height: 200 },
};

export const IDLE_TIMEOUT = 4000;
