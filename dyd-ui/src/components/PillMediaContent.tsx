import { useMediaStore } from "../stores/media-store";

function MiniArtwork({ artUrl }: { artUrl: string | null }) {
  if (!artUrl) {
    return (
      <svg
        width="20"
        height="20"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        style={{ color: "rgba(255,255,255,0.7)", flexShrink: 0 }}
        role="img"
        aria-label="Now playing"
      >
        <path d="M9 18V5l12-2v13" />
        <circle cx="6" cy="18" r="3" />
        <circle cx="18" cy="16" r="3" />
      </svg>
    );
  }
  return (
    <img
      src={artUrl}
      alt=""
      style={{ width: 22, height: 22, borderRadius: 6, objectFit: "cover", flexShrink: 0 }}
    />
  );
}

function PlayingWaves() {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "flex-end",
        gap: 2,
        height: 12,
        flexShrink: 0,
        color: "rgba(255,255,255,0.85)",
      }}
      role="img"
      aria-label="Playing"
    >
      {[0, 1, 2].map((i) => (
        <span key={i} className="dyd-wave" style={{ animationDelay: `${i * 0.15}s` }} />
      ))}
    </div>
  );
}

export default function PillMediaContent() {
  const media = useMediaStore((s) => s.media);
  if (!media || media.playbackStatus !== "playing") return null;

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        width: "100%",
        maxWidth: 230,
        minWidth: 0,
      }}
      role="status"
      aria-label={`Now playing: ${media.title}`}
    >
      <MiniArtwork artUrl={media.artUrl} />
      <div style={{ minWidth: 0, overflow: "hidden", flex: 1 }}>
        <span
          style={{
            display: "block",
            whiteSpace: "nowrap",
            overflow: "hidden",
            textOverflow: "ellipsis",
            color: "#fff",
            fontSize: 12,
            fontWeight: 600,
            lineHeight: 1.2,
          }}
        >
          {media.title}
        </span>
        <span
          style={{
            display: "block",
            whiteSpace: "nowrap",
            overflow: "hidden",
            textOverflow: "ellipsis",
            color: "rgba(255,255,255,0.6)",
            fontSize: 10,
            lineHeight: 1.2,
          }}
        >
          {media.artist || media.player}
        </span>
      </div>
      <PlayingWaves />
    </div>
  );
}