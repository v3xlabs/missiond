import { apiRequest } from "./api";
import { failure, outcome } from "./request";
import type { components } from "./schema.gen";

export type TabInfo = components["schemas"]["TabInfo"];

export const listTabs = async (): Promise<readonly TabInfo[]> => {
  const response = await apiRequest("/tabs", "get", {});

  if (response.status !== 200) {
    throw failure(response);
  }

  return response.data;
};

export const upsertTab = async (tabId: string, tab: components["schemas"]["UpsertTabRequest"]) =>
  outcome(await apiRequest("/tabs/{tab_id}", "put", {
    path: { tab_id: tabId },
    contentType: "application/json; charset=utf-8",
    data: tab,
  }));

export const deleteTab = async (tabId: string) =>
  outcome(await apiRequest("/tabs/{tab_id}", "delete", { path: { tab_id: tabId } }));

export const refreshTab = async (tabId: string) =>
  outcome(await apiRequest("/tabs/{tab_id}/refresh", "post", { path: { tab_id: tabId } }));

export const recreateTab = async (tabId: string) =>
  outcome(await apiRequest("/tabs/{tab_id}/recreate", "post", { path: { tab_id: tabId } }));
