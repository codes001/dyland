import { create } from "zustand";

export interface MediaState {
  player: string;
  title: string;
  artist: string;
  album: string;
  artUrl: string | null;
  length: number;
  position: number;
  playbackStatus: string;
}

export interface MediaStore {
  media: MediaState | null;
  artCache: Map<string, string>;
  setMedia: (payload: MediaState) => void;
  clearMedia: () => void;
  get isPlaying(): boolean;
}

export const useMediaStore = create<MediaStore>((set, get) => ({
  media: null,
  artCache: new Map(),

  setMedia: (payload: MediaState) => {
    const artCache = get().artCache;
    if (payload.artUrl && !artCache.has(payload.artUrl)) {
      const cache = new Map(artCache);
      cache.set(payload.artUrl, payload.artUrl);
      set({ media: payload, artCache: cache });
    } else {
      set({ media: payload });
    }
  },

  clearMedia: () => {
    set({ media: null });
  },

  get isPlaying(): boolean {
    const state = get();
    return state.media?.playbackStatus === "playing";
  },
}));
