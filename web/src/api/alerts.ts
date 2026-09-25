import { apiRequest } from "./api";
import { failure, outcome } from "./request";
import type { components } from "./schema.gen";

export type StingerInfo = components["schemas"]["StingerInfo"];

export const listStingers = async (): Promise<readonly StingerInfo[]> => {
  const response = await apiRequest("/stingers", "get", {});

  if (response.status !== 200) {
    throw failure(response);
  }

  return response.data;
};

export const raiseAlert = async (alert: components["schemas"]["NotifyRequest"]) =>
  outcome(await apiRequest("/notify", "post", {
    contentType: "application/json; charset=utf-8",
    data: alert,
  }));
