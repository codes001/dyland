import { create } from "zustand";
import type {
  BatteryPayload,
  BluetoothPayload,
  BrightnessPayload,
  VolumePayload,
  WifiPayload,
} from "../../../types/src/index";

export type StatusFeature = "battery" | "wifi" | "bluetooth" | "volume" | "brightness" | null;

export interface SystemStatus {
  battery: BatteryPayload | null;
  wifi: WifiPayload | null;
  bluetooth: BluetoothPayload | null;
  volume: VolumePayload | null;
  brightness: BrightnessPayload | null;
}

export interface SystemStore extends SystemStatus {
  selectedStatusFeature: StatusFeature;
  setBattery: (payload: BatteryPayload) => void;
  setWifi: (payload: WifiPayload) => void;
  setBluetooth: (payload: BluetoothPayload) => void;
  setVolume: (payload: VolumePayload) => void;
  setBrightness: (payload: BrightnessPayload) => void;
  selectStatusFeature: (feature: Exclude<StatusFeature, null>) => void;
  clearSelectedStatusFeature: () => void;
}

export const useSystemStore = create<SystemStore>((set) => ({
  battery: null,
  wifi: null,
  bluetooth: null,
  volume: null,
  brightness: null,
  selectedStatusFeature: null,

  setBattery: (payload: BatteryPayload) => set({ battery: payload }),
  setWifi: (payload: WifiPayload) => set({ wifi: payload }),
  setBluetooth: (payload: BluetoothPayload) => set({ bluetooth: payload }),
  setVolume: (payload: VolumePayload) => set({ volume: payload }),
  setBrightness: (payload: BrightnessPayload) => set({ brightness: payload }),

  selectStatusFeature: (feature: Exclude<StatusFeature, null>) => set({ selectedStatusFeature: feature }),
  clearSelectedStatusFeature: () => set({ selectedStatusFeature: null }),
}));
