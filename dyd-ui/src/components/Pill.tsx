import { useRef, useCallback, useEffect } from "react";
import { motion, AnimatePresence } from "motion/react";
import { usePillStore } from "../stores/pill-store";
import { PILL_SIZE, SPRING, DURATION, EASE, IDLE_TIMEOUT } from "../config/animation";
import { useGestureHandler } from "../hooks/useGestureHandler";
import type { PillState } from "../types/pill";

export type { PillState };

export interface PillProps {
  children?: React.ReactNode;
}

function getSize(state: PillState) {
  if (state === "hidden") return { width: 0, height: 0 };
  return PILL_SIZE[state] ?? PILL_SIZE.dormant;
}

export default function Pill({ children }: PillProps) {
  const ref = useRef<HTMLDivElement>(null);
  const state = usePillStore((s) => s.state);
  const isFullscreen = usePillStore((s) => s.isFullscreen);
  const transitionTo = usePillStore((s) => s.transitionTo);
  const resetIdleTimer = usePillStore((s) => s.resetIdleTimer);
  const setPreviousFullscreen = useRef(false);

  const handleClick = useCallback(() => {
    if (state === "active") transitionTo("expanded");
    resetIdleTimer();
  }, [state, transitionTo, resetIdleTimer]);

  const handleSwipeDismiss = useCallback(() => {
    if (state === "expanded") {
      transitionTo("active");
    } else if (state === "active") {
      transitionTo("dormant");
    }
  }, [state, transitionTo]);

  const handleLongPress = useCallback(() => {
    console.debug("long-press: context menu triggered");
  }, []);

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

  const size = getSize(state);
  const isInteractive = state === "active" || state === "expanded";
  const isHidden = state === "hidden";

  return (
    <motion.div
      ref={ref}
      tabIndex={isInteractive ? 0 : undefined}
      role={isInteractive ? "button" : undefined}
      aria-expanded={state === "expanded" ? true : undefined}
      onPointerDown={isInteractive ? onPointerDown : undefined}
      onMouseEnter={resetIdleTimer}
      animate={{
        width: size.width,
        minHeight: size.height,
        opacity: isHidden ? 0 : 1,
        y: isHidden ? -20 : 0,
        scaleX: 1,
        scaleY: 1,
      }}
      transition={
        state === "hidden" || state === "active" && isFullscreen
          ? { duration: DURATION.toHidden, ease: EASE.out }
          : SPRING.apple
      }
      className="pill"
      style={{
        position: "fixed",
        top: 0,
        left: "50%",
        x: "-50%",
        backgroundColor: "#000000",
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
            transition={{
              duration: DURATION.contentEntry,
              ease: EASE.out,
              delay: DURATION.contentStagger,
            }}
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
