import { motion } from "motion/react";
import { useMediaStore } from "../stores/media-store";
import { ipcClient } from "../ipc/client";

function sendMediaCommand(type: string, player: string) {
  const cmd = JSON.stringify({ type, payload: { player } });
  ipcClient.sendCommand(cmd);
}

export default function MediaControls() {
  const media = useMediaStore((s) => s.media);
  const isPlaying = media?.playbackStatus === "playing";

  if (!media) return null;

  const player = media.player;
  const disabled = !player;

  const btnClass =
    "flex items-center justify-center w-10 h-10 rounded-full transition-colors duration-150" +
    " enabled:hover:bg-white/10 enabled:cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed";

  return (
    <motion.div
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -8 }}
      transition={{ duration: 0.2, delay: 0.05 }}
      className="flex items-center justify-center gap-2 mt-2"
    >
      <button
        className={btnClass}
        disabled={disabled}
        onClick={() => sendMediaCommand("MediaPrevious", player)}
        aria-label="Previous track"
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
          <path d="M6 6h2v12H6zm3.5 6l8.5 6V6z" />
        </svg>
      </button>

      <button
        className={
          "flex items-center justify-center w-12 h-12 rounded-full transition-colors duration-150" +
          " enabled:hover:bg-white/10 enabled:cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
        }
        disabled={disabled}
        onClick={() => sendMediaCommand("MediaPlayPause", player)}
        aria-label={isPlaying ? "Pause" : "Play"}
      >
        {isPlaying ? (
          <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
            <path d="M6 4h4v16H6zm8 0h4v16h-4z" />
          </svg>
        ) : (
          <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
            <path d="M8 5v14l11-7z" />
          </svg>
        )}
      </button>

      <button
        className={btnClass}
        disabled={disabled}
        onClick={() => sendMediaCommand("MediaNext", player)}
        aria-label="Next track"
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
          <path d="M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z" />
        </svg>
      </button>
    </motion.div>
  );
}
