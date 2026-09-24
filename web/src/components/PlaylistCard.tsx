import { FiPlay, FiTrash2 } from "solid-icons/fi";
import { createSignal, For, Show, useContext } from "solid-js";

import type { PlaylistInfo } from "../api/playlists";
import { activatePlaylist, deletePlaylist, reorderTabs } from "../api/playlists";
import { DisplayContext } from "../app/display";
import { AddTabDialog } from "./AddTabDialog";
import { DANGER_ICON_BUTTON, SECONDARY_BUTTON } from "./controls";
import { TabCard } from "./TabCard";

export const PlaylistCard = (properties: { playlist: PlaylistInfo; }) => {
  const display = useContext(DisplayContext);
  const [draggedTabId, setDraggedTabId] = createSignal<string | null>(null);

  const tabs = () => display.playlistTabs().get(properties.playlist.playlist_id) ?? [];
  const isActive = () => display.status().current_playlist_id === properties.playlist.playlist_id;

  const move = (tabId: string, toIndex: number) => {
    const tabIds = tabs().map(tab => tab.tab_id);
    const fromIndex = tabIds.indexOf(tabId);

    if (fromIndex === -1 || fromIndex === toIndex || toIndex < 0 || toIndex >= tabIds.length) {
      return;
    }

    tabIds.splice(fromIndex, 1);
    tabIds.splice(toIndex, 0, tabId);

    void display.change(async () => reorderTabs(properties.playlist.playlist_id, tabIds), ["playlistTabs"]);
  };

  return (
    <section class="rounded-panel bg-surface" aria-label={properties.playlist.name}>
      <header class="flex flex-wrap items-center gap-x-3 gap-y-2 px-4 py-3">
        <div class="flex min-w-0 flex-1 flex-wrap items-baseline gap-x-3 gap-y-0.5">
          <h3 class="truncate text-sm font-semibold text-slate-900 dark:text-slate-100">{properties.playlist.name}</h3>
          <Show when={isActive()}>
            <span class="flex items-center gap-1.5 self-center text-xs font-medium text-emerald-700 dark:text-emerald-400">
              <span class="size-2 rounded-full bg-emerald-500" aria-hidden="true" />
              Playing
            </span>
          </Show>
          <Show when={properties.playlist.is_default}>
            <span class="text-xs text-slate-500 dark:text-slate-500">Default</span>
          </Show>
          <span class="text-xs text-slate-500 tabular-nums dark:text-slate-500">
            {`Every ${properties.playlist.interval}, ${properties.playlist.tab_count} ${properties.playlist.tab_count === 1 ? "tab" : "tabs"}`}
          </span>
        </div>
        <div class="flex shrink-0 items-center gap-2">
          <AddTabDialog playlistId={properties.playlist.playlist_id} playlistName={properties.playlist.name} />
          <Show when={!isActive()}>
            <button
              type="button"
              onClick={() => void display.change(async () => activatePlaylist(properties.playlist.playlist_id), ["status", "playlists"])}
              class={SECONDARY_BUTTON}
            >
              <FiPlay size={14} aria-hidden="true" />
              Play
            </button>
          </Show>
          <button
            type="button"
            onClick={() => void display.change(async () => deletePlaylist(properties.playlist.playlist_id), ["playlists", "status"])}
            aria-label={`Delete ${properties.playlist.name}`}
            title="Delete playlist"
            class={DANGER_ICON_BUTTON}
          >
            <FiTrash2 size={14} aria-hidden="true" />
          </button>
        </div>
      </header>

      <Show
        when={tabs().length > 0}
        fallback={<p class="px-4 pb-4 text-sm text-slate-500 dark:text-slate-500">No tabs yet.</p>}
      >
        <ol class="flex gap-4 overflow-x-auto px-4 pt-1 pb-4" aria-label={`Tabs of ${properties.playlist.name}, in play order`}>
          <For each={tabs()} keyed={tab => tab.tab_id}>
            {(tab, index) => (
              <TabCard
                tab={tab()}
                playlistId={properties.playlist.playlist_id}
                isOnScreen={isActive() && display.status().current_tab_id === tab().tab_id}
                isDragged={draggedTabId() === tab().tab_id}
                onDragStart={() => setDraggedTabId(tab().tab_id)}
                onDragEnd={() => setDraggedTabId(null)}
                onDrop={() => {
                  const dragged = draggedTabId();

                  if (dragged !== null) move(dragged, index());
                }}
                onMove={offset => move(tab().tab_id, index() + offset)}
              />
            )}
          </For>
        </ol>
      </Show>
    </section>
  );
};
