import { useMemo, useState } from "react";
import { resolveAppIconCached, evictIcon, type AppIcon } from "../../lib/notification-utils";

export interface AppIconProps {
  appIcon: string | null | undefined;
  appName: string | null | undefined;
  size: number;
}

function BellGlyph({ size }: { size: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      style={{ color: "rgba(255,255,255,0.85)" }}
    >
      <path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9" />
      <path d="M10.3 21a1.94 1.94 0 0 0 3.4 0" />
    </svg>
  );
}

function letterHue(appName: string): number {
  let sum = 0;
  for (let i = 0; i < appName.length; i++) sum += (appName.charCodeAt(i) || 0) * (i + 1);
  return sum % 360;
}

function LetterAvatar({ letter, appName, size }: { letter: string; appName: string; size: number }) {
  const hue = useMemo(() => letterHue(appName), [appName]);
  return (
    <div
      style={{
        width: size,
        height: size,
        borderRadius: size >= 24 ? 8 : 6,
        backgroundColor: `hsl(${hue}, 45%, 40%)`,
        color: "#fff",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        fontSize: Math.round(size * 0.55),
        fontWeight: 600,
        flexShrink: 0,
      }}
    >
      {letter}
    </div>
  );
}

export default function AppIcon({ appIcon, appName, size }: AppIconProps) {
  const [imageFailed, setImageFailed] = useState(false);
  const resolved: AppIcon = useMemo(() => {
    if (imageFailed) return { kind: "letter", letter: appName?.charAt(0)?.toUpperCase() ?? "?", appName: appName ?? "" };
    return resolveAppIconCached(appIcon, appName);
  }, [imageFailed, appIcon, appName]);

  if (resolved.kind === "image") {
    return (
      <div
        style={{
          width: size,
          height: size,
          borderRadius: Math.round(size * 0.35),
          overflow: "hidden",
          flexShrink: 0,
          backgroundColor: "rgba(255,255,255,0.1)",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        <img
          src={resolved.src}
          alt=""
          width={size}
          height={size}
          onError={() => {
            evictIcon(appIcon, appName);
            setImageFailed(true);
          }}
          style={{ objectFit: "cover", width: size, height: size }}
        />
      </div>
    );
  }

  if (resolved.kind === "letter") {
    return <LetterAvatar letter={resolved.letter} appName={resolved.appName} size={size} />;
  }

  return (
    <div
      style={{
        width: size,
        height: size,
        borderRadius: Math.round(size * 0.35),
        backgroundColor: "rgba(255,255,255,0.1)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        flexShrink: 0,
      }}
    >
      <BellGlyph size={Math.round(size * 0.6)} />
    </div>
  );
}

export { BellGlyph };