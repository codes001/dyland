import { motion, AnimatePresence } from "motion/react";

export interface ContextMenuItem {
  label: string;
  action: () => void;
}

export interface ContextMenuProps {
  isOpen: boolean;
  items: ContextMenuItem[];
  onClose: () => void;
}

export default function ContextMenu({ isOpen, items, onClose }: ContextMenuProps) {
  return (
    <AnimatePresence>
      {isOpen && (
        <>
          <div
            style={{
              position: "fixed",
              inset: 0,
              zIndex: 10001,
            }}
            onClick={onClose}
          />
          <motion.div
            initial={{ opacity: 0, scale: 0.9, y: -4 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.9, y: -4 }}
            transition={{ duration: 0.15 }}
            data-island-hit=""
            style={{
              position: "absolute",
              top: 58,
              left: "50%",
              x: "-50%",
              marginTop: 4,
              backgroundColor: "#1c1c1e",
              borderRadius: 12,
              overflow: "hidden",
              minWidth: 140,
              zIndex: 10002,
              boxShadow: "0 8px 24px rgba(0,0,0,0.4)",
            }}
            role="menu"
          >
            {items.map((item, i) => (
              <button
                key={i}
                onClick={() => {
                  item.action();
                  onClose();
                }}
                style={{
                  width: "100%",
                  padding: "10px 16px",
                  background: "none",
                  border: "none",
                  color: "#ffffff",
                  fontSize: 13,
                  textAlign: "left",
                  cursor: "pointer",
                }}
                onMouseEnter={(e) => (e.currentTarget.style.backgroundColor = "rgba(255,255,255,0.08)")}
                onMouseLeave={(e) => (e.currentTarget.style.backgroundColor = "transparent")}
                role="menuitem"
              >
                {item.label}
              </button>
            ))}
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}
