import { useRef, useCallback } from "react";

interface GestureOptions {
  onClick?: () => void;
  onSwipe?: (direction: "up" | "down") => void;
  onLongPress?: () => void;
  swipeThreshold?: number;
  swipeVelocityThreshold?: number;
  longPressDelay?: number;
  moveThreshold?: number;
}

type GestureRecognizer = "idle" | "pending" | "click" | "swipe" | "long-press";

export function useGestureHandler({
  onClick,
  onSwipe,
  onLongPress,
  swipeThreshold = 30,
  swipeVelocityThreshold = 200,
  longPressDelay = 500,
  moveThreshold = 10,
}: GestureOptions) {
  const recognizer = useRef<GestureRecognizer>("idle");
  const startY = useRef(0);
  const startX = useRef(0);
  const startTime = useRef(0);
  const longPressTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const cancelLongPress = useCallback(() => {
    if (longPressTimer.current) {
      clearTimeout(longPressTimer.current);
      longPressTimer.current = null;
    }
  }, []);

  const onPointerDown = useCallback(
    (e: React.PointerEvent) => {
      recognizer.current = "pending";
      startY.current = e.clientY;
      startX.current = e.clientX;
      startTime.current = Date.now();

      const pointerId = e.pointerId;

      const onPointerMove = (moveEvent: PointerEvent) => {
        if (moveEvent.pointerId !== pointerId) return;
        const dy = moveEvent.clientY - startY.current;
        const dx = moveEvent.clientX - startX.current;

        if (Math.abs(dy) > moveThreshold || Math.abs(dx) > moveThreshold) {
          cancelLongPress();
          if (recognizer.current === "pending") {
            recognizer.current = "swipe";
          }
        }

        if (recognizer.current === "swipe") {
          const clamped = dy > 0
            ? dy <= swipeThreshold
              ? dy
              : swipeThreshold + (dy - swipeThreshold) * 0.3
            : dy * 0.3;
          (e.currentTarget as HTMLElement).style.transform = `translateX(-50%) translateY(${clamped}px)`;
        }
      };

      const onPointerUp = (upEvent: PointerEvent) => {
        if (upEvent.pointerId !== pointerId) return;
        cancelLongPress();

        document.removeEventListener("pointermove", onPointerMove);
        document.removeEventListener("pointerup", onPointerUp);

        const dy = upEvent.clientY - startY.current;
        const dt = Date.now() - startTime.current;
        const velocity = Math.abs(dy) / (dt || 1) * 1000;

        if (recognizer.current === "swipe") {
          (e.currentTarget as HTMLElement).style.transform = "";
          if (dy > swipeThreshold || velocity > swipeVelocityThreshold) {
            onSwipe?.("down");
          }
          recognizer.current = "idle";
          return;
        }

        if (dt < 300 && Math.abs(dy) < moveThreshold) {
          recognizer.current = "click";
          onClick?.();
        }

        recognizer.current = "idle";
      };

      document.addEventListener("pointermove", onPointerMove);
      document.addEventListener("pointerup", onPointerUp);

      longPressTimer.current = setTimeout(() => {
        if (recognizer.current === "pending") {
          recognizer.current = "long-press";
          document.removeEventListener("pointermove", onPointerMove);
          document.removeEventListener("pointerup", onPointerUp);
          onLongPress?.();
        }
      }, longPressDelay);
    },
    [onClick, onSwipe, onLongPress, swipeThreshold, swipeVelocityThreshold, longPressDelay, moveThreshold, cancelLongPress],
  );

  return { onPointerDown };
}
