import { useRef, useCallback, useEffect } from "react";
import { motion, AnimatePresence } from "motion/react";
import { usePillStore } from "../stores/pill-store";
import { useNotificationStore } from "../stores/notification-store";
import { useConfigStore } from "../stores/config-store";
import { PILL_SIZE, SPRING, DURATION, EASE } from "../config/animation";
import { useGestureHandler } from "../hooks/useGestureHandler";
import type { PillState } from "../types/pill";

export type { PillState };

export interface PillProps {
  children?: React.ReactNode;
}

function getSize(state: PillState, hasNotification: boolean) {
  if (state === "hidden") return { width: 0, height: 0 };
  if (hasNotification && (state === "dormant" || state === "active")) {
    return PILL_SIZE.notification;
  }
  return PILL_SIZE[state] ?? PILL_SIZE.dormant;
}

export default function Pill({ children, onLongPress }: PillProps & { onLongPress?: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  const state = usePillStore((s) => s.state);
  const isFullscreen = usePillStore((s) => s.isFullscreen);
  const transitionTo = usePillStore((s) => s.transitionTo);
  const resetIdleTimer = usePillStore((s) => s.resetIdleTimer);
  const hasNotification = useNotificationStore((s) => s.notification !== null);
  const reduceMotion = useConfigStore((s) => s.config?.accessibility?.reduce_motion);
  const setPreviousFullscreen = useRef(false);

  const handleClick = useCallback(() => {
    if (state === "active" || (state === "dormant" && hasNotification)) transitionTo("expanded");
    resetIdleTimer();
  }, [state, hasNotification, transitionTo, resetIdleTimer]);

  const handleSwipeDismiss = useCallback(() => {
    if (state === "expanded") {
      transitionTo("active");
    } else if (state === "active") {
      transitionTo("dormant");
    }
  }, [state, transitionTo]);

  const handleLongPress = useCallback(() => {
    if (onLongPress) {
      onLongPress();
    } else {
      console.debug("long-press: context menu triggered");
    }
  }, [onLongPress]);

  const { onPointerDown } = useGestureHandler({
    onClick: handleClick,
    onSwipe: handleSwipeDismiss,
    onLongPress: handleLongPress,
  });

  useEffect(() => {
    if (isFullscreen !== setPreviousFullscreen.current) {
      setPreviousFullscreen.current = isFullscreen;
      usePillStore.getState().handleFullscreenChange(isFullscreen);
    }
  }, [isFullscreen]);

  const size = getSize(state, hasNotification);
  const isInteractive =
    state === "active" || state === "expanded" || (state === "dormant" && hasNotification);
  const isHidden = state === "hidden";

  const transitionSpec = reduceMotion === true
    ? { duration: 0 }
    : state === "hidden" || (state === "active" && isFullscreen)
    ? { duration: DURATION.toHidden, ease: EASE.out }
    : SPRING.apple;

  return (
    <motion.div
      ref={ref}
      tabIndex={isInteractive ? 0 : undefined}
      role={isInteractive ? "button" : "status"}
      aria-live="polite"
      aria-label="Dynamic Island"
      aria-expanded={state === "expanded" ? true : undefined}
      onPointerDown={isInteractive ? onPointerDown : undefined}
      onMouseEnter={resetIdleTimer}
      whileHover={isInteractive && reduceMotion !== true ? { scale: 1.02 } : undefined}
      whileTap={isInteractive && reduceMotion !== true ? { scale: 0.98 } : undefined}
      animate={{
        x: "-50%",
        width: size.width,
        minHeight: size.height,
        opacity: isHidden ? 0 : 1,
        y: isHidden ? -20 : 0,
        scaleX: 1,
        scaleY: 1,
      }}
      transition={transitionSpec}
      className="pill"
      style={{
        position: "fixed",
        top: 10,
        left: "50%",
        x: "-50%",
        backgroundColor: "var(--pill-bg)",
        color: "var(--pill-text)",
        border: "1px solid var(--pill-border)",
        borderRadius: 18,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        overflow: "hidden",
        cursor: isInteractive ? "pointer" : "default",
        pointerEvents: isHidden ? "none" : "auto",
        willChange: "transform, opacity, width, height",
        zIndex: 9999,
      }}
    >
      <AnimatePresence mode="popLayout">
        {!isHidden && (
          <motion.div
            key={state}
            initial={{ y: 10, opacity: 0 }}
            animate={{ y: 0, opacity: 1 }}
            exit={{ y: -10, opacity: 0 }}
            transition={
              reduceMotion === true
                ? { duration: 0 }
                : {
                    duration: DURATION.contentEntry,
                    ease: EASE.out,
                    delay: DURATION.contentStagger,
                  }
            }
            style={{
              width: "100%",
              height: "100%",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              padding: "0 12px",
            }}
          >
            {children}
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );
}
