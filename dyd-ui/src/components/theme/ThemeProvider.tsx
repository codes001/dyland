import { useEffect, useState, type ReactNode } from "react";
import { useConfigStore } from "../../stores/config-store";

interface Props {
  children: ReactNode;
}

export default function ThemeProvider({ children }: Props) {
  const theme = useConfigStore((s) => s.config.appearance.theme);
  const accessibility = useConfigStore((s) => s.config.accessibility);
  const [systemIsDark, setSystemIsDark] = useState<boolean>(() =>
    window.matchMedia("(prefers-color-scheme: dark)").matches
  );

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const listener = (e: MediaQueryListEvent) => setSystemIsDark(e.matches);
    media.addEventListener("change", listener);
    return () => media.removeEventListener("change", listener);
  }, []);

  const activeTheme = theme === "system" ? (systemIsDark ? "dark" : "light") : theme;

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", activeTheme);
  }, [activeTheme]);

  useEffect(() => {
    const root = document.documentElement;
    if (accessibility.font_scale != null) {
      root.style.setProperty("--font-scale", `${accessibility.font_scale}`);
    } else {
      root.style.removeProperty("--font-scale");
    }

    if (accessibility.reduce_motion === true) {
      root.setAttribute("data-reduce-motion", "true");
    } else {
      root.removeAttribute("data-reduce-motion");
    }

    if (accessibility.high_contrast === true) {
      root.setAttribute("data-high-contrast", "true");
    } else {
      root.removeAttribute("data-high-contrast");
    }
  }, [accessibility]);

  return <>{children}</>;
}
