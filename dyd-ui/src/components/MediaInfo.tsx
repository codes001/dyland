import { useState, useCallback, useRef } from "react";
import { motion } from "motion/react";
import { useMediaStore } from "../stores/media-store";
import { ipcClient } from "../ipc/client";

function MusicNoteIcon() {
  return (
    <svg
      width="24"
      height="24"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      className="text-white/40"
    >
      <path d="M9 18V5l12-2v13" />
      <circle cx="6" cy="18" r="3" />
      <circle cx="18" cy="16" r="3" />
    </svg>
  );
}

function MarqueeText({ text, className }: { text: string; className?: string }) {
  const [overflows, setOverflows] = useState(false);

  const handleRef = useCallback((el: HTMLSpanElement | null) => {
    if (el) {
      setOverflows(el.scrollWidth > el.clientWidth);
    }
  }, []);

  const style: React.CSSProperties = {
    display: "block",
    overflow: "hidden",
    whiteSpace: "nowrap",
    textOverflow: overflows ? undefined : "ellipsis",
  };

  return (
    <span ref={handleRef} className={className} style={style}>
      {overflows ? (
        <span className="marquee">
          <span>{text}</span>
          <span>&nbsp;&nbsp;&nbsp;{text}</span>
        </span>
      ) : (
        text
      )}
    </span>
  );
}

function ArtImage({ artUrl }: { artUrl: string | null }) {
  const [loaded, setLoaded] = useState(false);
  const [error, setError] = useState(false);

  if (!artUrl || error) {
    return (
      <div className="flex items-center justify-center w-12 h-12 rounded-lg bg-white/10 shrink-0">
        <MusicNoteIcon />
      </div>
    );
  }

  return (
    <div className="relative w-12 h-12 rounded-lg overflow-hidden shrink-0">
      {!loaded && (
        <div className="absolute inset-0 animate-pulse bg-white/10" />
      )}
      <img
        src={artUrl}
        alt=""
        className={`w-full h-full object-cover transition-opacity duration-300 ${loaded ? "opacity-100" : "opacity-0"}`}
        onLoad={() => setLoaded(true)}
        onError={() => setError(true)}
      />
    </div>
  );
}

function ProgressBar({ position, length }: { position: number; length: number }) {
  if (length <= 0) return null;

  const pct = Math.min((position / length) * 100, 100);

  return (
    <div className="w-full h-0.5 bg-white/10 rounded-full overflow-hidden mt-1.5">
      <motion.div
        className="h-full bg-white/40 rounded-full"
        initial={{ width: 0 }}
        animate={{ width: `${pct}%` }}
        transition={{ duration: 0.3, ease: "easeOut" }}
      />
    </div>
  );
}

function useLongPress(callback: () => void, ms = 500) {
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const onPointerDown = useCallback(() => {
    timerRef.current = setTimeout(() => {
      callback();
    }, ms);
  }, [callback, ms]);

  const onPointerUp = useCallback(() => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
  }, []);

  const onPointerLeave = useCallback(() => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
  }, []);

  return { onPointerDown, onPointerUp, onPointerLeave };
}

export default function MediaInfo() {
  const media = useMediaStore((s) => s.media);

  const handleCyclePlayer = useCallback(() => {
    if (media?.player) {
      const cmd = JSON.stringify({ type: "CyclePlayer", payload: { player: media.player } });
      ipcClient.sendCommand(cmd);
    }
  }, [media?.player]);

  const longPress = useLongPress(handleCyclePlayer, 500);

  if (!media) return null;

  return (
    <motion.div
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -8 }}
      transition={{ duration: 0.2 }}
      className="flex items-start gap-3 w-full px-1 cursor-default select-none"
      {...longPress}
    >
      <ArtImage artUrl={media.artUrl} />
      <div className="flex flex-col min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="text-[10px] font-medium text-white/50 uppercase tracking-wider bg-white/5 rounded-full px-2 py-0.5">
            {media.player}
          </span>
        </div>
        <MarqueeText
          text={media.title}
          className="text-sm font-semibold text-white leading-tight"
        />
        <span className="text-xs text-white/60 truncate">
          {media.artist}{media.album ? ` · ${media.album}` : ""}
        </span>
        <ProgressBar position={media.position} length={media.length} />
      </div>
    </motion.div>
  );
}
