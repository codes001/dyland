export const SPRING = {
  apple: { type: "spring" as const, stiffness: 350, damping: 25, mass: 0.8 },
  expand: { type: "spring" as const, stiffness: 300, damping: 25, mass: 1 },
  collapse: { type: "spring" as const, stiffness: 350, damping: 30, mass: 0.9 },
  quick: { type: "spring" as const, stiffness: 400, damping: 28, mass: 0.7 },
};

export const EASE = {
  out: [0.0, 0.0, 0.2, 1] as [number, number, number, number],
  in: [0.4, 0.0, 1, 1] as [number, number, number, number],
};

export const DURATION = {
  dormantToActive: 0.3,
  activeToDormant: 0.2,
  activeToExpanded: 0.4,
  expandedToActive: 0.3,
  toHidden: 0.1,
  hiddenToPrev: 0.15,
  contentEntry: 0.2,
  contentExit: 0.15,
  contentStagger: 0.05,
  hoverFade: 0.15,
  hoverExit: 0.2,
};

export const PILL_SIZE = {
  dormant: { width: 126, height: 36 },
  active: { width: 180, height: 36 },
  notification: { width: 200, height: 36 },
  expanded: { width: 380, height: 200 },
};

export const IDLE_TIMEOUT = 4000;
