import { apiRequest } from "./api";
import { failure, outcome } from "./request";
import type { components } from "./schema.gen";
import type { TabInfo } from "./tabs";

export type PlaylistInfo = components["schemas"]["PlaylistInfo"];

const JSON_BODY = "application/json; charset=utf-8";

export const listPlaylists = async (): Promise<readonly PlaylistInfo[]> => {
  const response = await apiRequest("/playlists", "get", {});

  if (response.status !== 200) {
    throw failure(response);
  }

  return response.data;
};

export const listPlaylistTabs = async (playlistId: string): Promise<readonly TabInfo[]> => {
  const response = await apiRequest("/playlists/{playlist_id}/tabs", "get", {
    path: { playlist_id: playlistId },
  });

  if (response.status !== 200) {
    throw failure(response);
  }

  return response.data;
};

export const createPlaylist = async (playlist: components["schemas"]["CreatePlaylistRequest"]) =>
  outcome(await apiRequest("/playlists", "post", { contentType: JSON_BODY, data: playlist }));

export const deletePlaylist = async (playlistId: string) =>
  outcome(await apiRequest("/playlists/{playlist_id}", "delete", { path: { playlist_id: playlistId } }));

export const activatePlaylist = async (playlistId: string) =>
  outcome(await apiRequest("/playlists/{playlist_id}/activate", "post", { path: { playlist_id: playlistId } }));

export const activateTab = async (playlistId: string, tabId: string) =>
  outcome(await apiRequest("/playlists/{playlist_id}/tabs/{tab_id}/activate", "post", {
    path: { playlist_id: playlistId, tab_id: tabId },
  }));

export const addTabToPlaylist = async (playlistId: string, tabId: string) =>
  outcome(await apiRequest("/playlists/{playlist_id}/tabs/{tab_id}", "put", {
    path: { playlist_id: playlistId, tab_id: tabId },
  }));

export const removeTabFromPlaylist = async (playlistId: string, tabId: string) =>
  outcome(await apiRequest("/playlists/{playlist_id}/tabs/{tab_id}", "delete", {
    path: { playlist_id: playlistId, tab_id: tabId },
  }));

export const reorderTabs = async (playlistId: string, tabIds: readonly string[]) =>
  outcome(await apiRequest("/playlists/{playlist_id}/reorder", "put", {
    path: { playlist_id: playlistId },
    contentType: JSON_BODY,
    data: { tab_ids: [...tabIds] },
  }));

export const setTabEnabled = async (playlistId: string, tabId: string, isEnabled: boolean) =>
  outcome(await apiRequest("/playlists/{playlist_id}/tabs/{tab_id}/enabled", "put", {
    path: { playlist_id: playlistId, tab_id: tabId },
    contentType: JSON_BODY,
    data: { enabled: isEnabled },
  }));
