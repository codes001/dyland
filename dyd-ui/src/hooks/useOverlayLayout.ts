import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function publishInputRegions() {
  const regions: { x: number; y: number; width: number; height: number }[] = [];
  document.querySelectorAll<HTMLElement>("[data-island-hit]").forEach((node) => {
    const style = getComputedStyle(node);
    if (style.visibility === "hidden" || style.pointerEvents === "none") return;
    const opacity = Number.parseFloat(style.opacity);
    if (Number.isFinite(opacity) && opacity < 0.05) return;
    const rect = node.getBoundingClientRect();
    if (rect.width < 1 || rect.height < 1) return;
    regions.push({
      x: rect.x,
      y: rect.y,
      width: rect.width,
      height: rect.height,
    });
  });
  void invoke("set_input_regions", { regions }).catch(() => {});
}

export function useOverlayLayout(visible: boolean, tall: boolean) {
  useEffect(() => {
    if (!inTauri()) return;
    document.documentElement.dataset.tauri = "true";
    void invoke("set_overlay_mode", { visible, tall }).catch(() => {});
  }, [visible, tall]);

  useEffect(() => {
    if (!inTauri()) return;
    let frame = 0;
    const request = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(publishInputRegions);
    };
    const resize = new ResizeObserver(request);
    const observe = () => {
      resize.disconnect();
      document.querySelectorAll("[data-island-hit]").forEach((node) => resize.observe(node));
    };
    const mutations = new MutationObserver(() => {
      observe();
      request();
    });
    observe();
    mutations.observe(document.body, {
      subtree: true,
      childList: true,
      attributes: true,
      attributeFilter: ["style", "class"],
    });
    request();
    return () => {
      cancelAnimationFrame(frame);
      resize.disconnect();
      mutations.disconnect();
    };
  }, []);
}
