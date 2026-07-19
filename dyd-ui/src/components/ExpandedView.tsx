import { useEffect, useRef } from "react";
import { motion } from "motion/react";
import { usePillStore } from "../stores/pill-store";
import { SPRING, DURATION } from "../config/animation";

export interface ExpandedViewProps {
  children: React.ReactNode;
  onClose?: () => void;
}

function supportsBackdropBlur(): boolean {
  return CSS.supports("backdrop-filter", "blur(1px)");
}

export default function ExpandedView({ children, onClose }: ExpandedViewProps) {
  const state = usePillStore((s) => s.state);
  const transitionTo = usePillStore((s) => s.transitionTo);
  const ref = useRef<HTMLDivElement>(null);

  const isVisible = state === "expanded";
  const hasBlur = supportsBackdropBlur();

  useEffect(() => {
    if (!isVisible) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        transitionTo("active");
        onClose?.();
      }
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [isVisible, transitionTo, onClose]);

  if (!isVisible) return null;

  return (
    <motion.div
      ref={ref}
      initial={{ opacity: 0, scale: 0.95, y: -5 }}
      animate={{ opacity: 1, scale: 1, y: 0 }}
      exit={{ opacity: 0, scale: 0.95, y: -5 }}
      transition={SPRING.expand}
      onClick={(e) => e.stopPropagation()}
      style={{
        position: "absolute",
        top: "100%",
        left: "50%",
        x: "-50%",
        marginTop: 8,
        width: 380,
        minHeight: 200,
        borderRadius: 18,
        overflow: "hidden",
        ...(hasBlur
          ? {
              backdropFilter: "blur(20px)",
              WebkitBackdropFilter: "blur(20px)",
              backgroundColor: "rgba(0, 0, 0, 0.75)",
            }
          : {
              backgroundColor: "rgba(0, 0, 0, 0.6)",
            }),
        color: "#ffffff",
        padding: 16,
        zIndex: 10000,
        willChange: "transform, opacity",
      }}
    >
      {children}
    </motion.div>
  );
}
