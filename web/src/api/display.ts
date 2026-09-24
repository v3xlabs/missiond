import { apiRequest } from "./api";
import { failure, outcome } from "./request";
import type { components } from "./schema.gen";

export type DeviceStatus = components["schemas"]["DeviceStatus"];

const PLAYBACK_PATHS = {
  next: "/playback/next",
  previous: "/playback/previous",
  pause: "/playback/pause",
  resume: "/playback/resume",
} as const;

export type PlaybackAction = keyof typeof PLAYBACK_PATHS;

export const getStatus = async (): Promise<DeviceStatus> => {
  const response = await apiRequest("/status", "get", {});

  if (response.status !== 200) {
    throw failure(response);
  }

  return response.data;
};

export const playback = async (action: PlaybackAction) =>
  outcome(await apiRequest(PLAYBACK_PATHS[action], "post", {}));

export const setScreenPower = async (isOn: boolean) =>
  outcome(await apiRequest("/display/power/{on}", "post", { path: { on: isOn } }));
