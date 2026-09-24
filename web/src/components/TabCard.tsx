import { FiExternalLink, FiRefreshCw, FiRotateCw, FiTrash2 } from "solid-icons/fi";
import { TbOutlineGripVertical } from "solid-icons/tb";
import { Show, useContext } from "solid-js";

import { activateTab, removeTabFromPlaylist, setTabEnabled } from "../api/playlists";
import type { TabInfo } from "../api/tabs";
import { recreateTab, refreshTab } from "../api/tabs";
import { DisplayContext } from "../app/display";
import { DANGER_ICON_BUTTON, ICON_BUTTON } from "./controls";
import { TabPreview } from "./TabPreview";

export const TabCard = (properties: {
  tab: TabInfo;
  playlistId: string;
  isOnScreen: boolean;
  isDragged: boolean;
  onDragStart: () => void;
  onDragEnd: () => void;
  onDrop: () => void;
  /** Moves the tab by this many places, for the keyboard. */
  onMove: (offset: number) => void;
}) => {
  const display = useContext(DisplayContext);

  return (
    <li
      draggable="true"
      onDragStart={(event) => {
        // Firefox starts no drag without data.
        event.dataTransfer?.setData("text/plain", properties.tab.tab_id);

        if (event.dataTransfer !== null) event.dataTransfer.effectAllowed = "move";

        properties.onDragStart();
      }}
      onDragEnd={() => properties.onDragEnd()}
      onDragOver={event => event.preventDefault()}
      onDrop={(event) => {
        event.preventDefault();
        properties.onDrop();
      }}
      class={["w-64 shrink-0 space-y-2", { "opacity-40": properties.isDragged, "opacity-60": !properties.tab.enabled }]}
    >
      <button
        type="button"
        onClick={() => void display.change(async () => activateTab(properties.playlistId, properties.tab.tab_id), ["status"])}
        title="Put this tab on screen"
        class={[
          "relative block aspect-video w-full overflow-hidden rounded-control bg-raised",
          { "ring-2 ring-emerald-500 ring-offset-2 ring-offset-surface": properties.isOnScreen },
        ]}
      >
        <Show
          when={properties.tab.url !== undefined}
          fallback={(
            <span class="absolute inset-0 flex items-center justify-center px-3 text-center text-xs text-slate-500 dark:text-slate-400">
              A camera plays outside the browser and has no preview
            </span>
          )}
        >
          <TabPreview tabId={properties.tab.tab_id} />
        </Show>
      </button>

      <div class="flex items-start gap-1">
        <button
          type="button"
          onKeyDown={(event) => {
            if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") {
              return;
            }

            event.preventDefault();
            properties.onMove(event.key === "ArrowLeft" ? -1 : 1);
          }}
          aria-label={`Reorder ${properties.tab.name}`}
          aria-keyshortcuts="ArrowLeft ArrowRight"
          title="Drag to reorder, or focus and use the arrow keys"
          class={[ICON_BUTTON, "cursor-grab"]}
        >
          <TbOutlineGripVertical size={14} aria-hidden="true" />
        </button>
        <div class="min-w-0 flex-1">
          <h3 class="truncate text-sm font-medium text-slate-900 dark:text-slate-100">{properties.tab.name}</h3>
          <p class="truncate text-xs text-slate-500 dark:text-slate-500">
            <Show when={properties.isOnScreen} fallback={properties.tab.url ?? "Camera"}>
              <span class="text-emerald-700 dark:text-emerald-400">On screen</span>
            </Show>
          </p>
        </div>
        <button
          type="button"
          role="switch"
          aria-checked={properties.tab.enabled ? "true" : "false"}
          aria-label={`Play ${properties.tab.name} in this playlist`}
          title={properties.tab.enabled ? "In rotation" : "Skipped in rotation"}
          onClick={() => void display.change(
            async () => setTabEnabled(properties.playlistId, properties.tab.tab_id, !properties.tab.enabled),
            ["playlistTabs"],
          )}
          class={[
            "mt-1 flex h-5 w-9 shrink-0 items-center rounded-full p-0.5 transition-colors",
            properties.tab.enabled ? "bg-emerald-600" : "bg-raised-hover",
          ]}
        >
          <span class={["size-4 rounded-full bg-white shadow-sm transition-transform", { "translate-x-4": properties.tab.enabled }]} />
        </button>
      </div>

      <div class="flex items-center gap-1">
        <Show when={properties.tab.url}>
          {url => (
            <>
              <button
                type="button"
                onClick={() => void display.change(async () => refreshTab(properties.tab.tab_id), [])}
                aria-label="Reload the page"
                title="Reload the page"
                class={ICON_BUTTON}
              >
                <FiRefreshCw size={14} aria-hidden="true" />
              </button>
              <button
                type="button"
                onClick={() => void display.change(async () => recreateTab(properties.tab.tab_id), [])}
                aria-label="Close and reopen the page"
                title="Close and reopen the page"
                class={ICON_BUTTON}
              >
                <FiRotateCw size={14} aria-hidden="true" />
              </button>
              <a
                href={url()}
                target="_blank"
                rel="noreferrer"
                aria-label="Open in this browser"
                title="Open in this browser"
                class={ICON_BUTTON}
              >
                <FiExternalLink size={14} aria-hidden="true" />
              </a>
            </>
          )}
        </Show>
        <span class="flex-1" />
        <button
          type="button"
          onClick={() => void display.change(
            async () => removeTabFromPlaylist(properties.playlistId, properties.tab.tab_id),
            ["playlistTabs", "playlists"],
          )}
          aria-label="Remove from this playlist"
          title="Remove from this playlist"
          class={DANGER_ICON_BUTTON}
        >
          <FiTrash2 size={14} aria-hidden="true" />
        </button>
      </div>
    </li>
  );
};
