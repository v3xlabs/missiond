import type { SourceAccessor } from "solid-js";
import { createContext, createMemo, onSettled, refresh } from "solid-js";

import type { StingerInfo } from "../api/alerts";
import { listStingers } from "../api/alerts";
import { baseUrl } from "../api/api";
import type { DeviceStatus } from "../api/display";
import { getStatus } from "../api/display";
import { notify } from "../api/notices";
import type { PlaylistInfo } from "../api/playlists";
import { listPlaylists, listPlaylistTabs } from "../api/playlists";
import type { Outcome } from "../api/request";
import type { TabInfo } from "../api/tabs";
import { listTabs } from "../api/tabs";

type Source = "status" | "playlists" | "playlistTabs" | "tabs";

export type Display = {
  status: SourceAccessor<DeviceStatus>;
  playlists: SourceAccessor<readonly PlaylistInfo[]>;
  /** Each playlist's tabs in play order, by playlist id. */
  playlistTabs: SourceAccessor<ReadonlyMap<string, readonly TabInfo[]>>;
  tabs: SourceAccessor<readonly TabInfo[]>;
  stingers: SourceAccessor<readonly StingerInfo[]>;
  /** Runs a write and refetches what it changed. A failure comes back for the caller to show. */
  apply: (write: () => Promise<Outcome>, affected: readonly Source[]) => Promise<Outcome>;
  /** Like `apply`, for a control with no place of its own to say it failed, so a failure becomes a notice. */
  change: (write: () => Promise<Outcome>, affected: readonly Source[]) => Promise<void>;
};

export const DisplayContext = createContext<Display>();

const isSameList = (one: readonly string[], other: readonly string[]) =>
  one.length === other.length && one.every((value, index) => value === other[index]);

export const createDisplay = (): Display => {
  const status = createMemo(() => getStatus());
  const playlists = createMemo(() => listPlaylists());
  const tabs = createMemo(() => listTabs());
  const stingers = createMemo(() => listStingers());

  // Activating a playlist refetches the list, but only a playlist coming or going changes whose
  // tabs there are to load.
  const playlistIds = createMemo(() => playlists().map(playlist => playlist.playlist_id), { equals: isSameList });
  const playlistTabs = createMemo(async () => new Map(await Promise.all(
    playlistIds().map(async (playlistId): Promise<[string, readonly TabInfo[]]> =>
      [playlistId, await listPlaylistTabs(playlistId)]),
  )));

  const sources: Record<Source, SourceAccessor<unknown>> = { status, playlists, playlistTabs, tabs };

  // The daemon announces every change to what is on screen, including the rotation's own steps,
  // which no request from this page would otherwise pick up. The frame is only a signal: it
  // carries the access of a connection opened without the admin key, so the status is fetched
  // again with the key rather than read from the frame.
  onSettled(() => {
    const events = new EventSource(new URL("events", baseUrl));

    events.addEventListener("state", () => void refresh(status));

    return () => events.close();
  });

  const apply = async (write: () => Promise<Outcome>, affected: readonly Source[]): Promise<Outcome> => {
    try {
      const result = await write();

      if (!result.ok) return result;
    }
    catch (error) {
      return { ok: false, message: error instanceof Error ? error.message : "The request did not complete." };
    }

    for (const source of affected) {
      void refresh(sources[source]);
    }

    return { ok: true };
  };

  const change = async (write: () => Promise<Outcome>, affected: readonly Source[]) => {
    const result = await apply(write, affected);

    if (!result.ok) notify(result.message);
  };

  return { status, playlists, playlistTabs, tabs, stingers, apply, change };
};
