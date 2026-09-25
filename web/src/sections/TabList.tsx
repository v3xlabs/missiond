import { FiPlay, FiTrash2 } from "solid-icons/fi";
import { Errored, For, Loading, Show, useContext } from "solid-js";

import { deleteTab, showTab } from "../api/tabs";
import { DisplayContext } from "../app/display";
import { DANGER_ICON_BUTTON, EMPTY_PANEL, ICON_BUTTON, SECTION_HEADING } from "../components/controls";
import { RegionFailure, RegionPending } from "../components/Region";

/**
 * Every configured tab, whether or not a playlist uses it. A camera raised by an alert belongs to
 * no playlist, and the playlist cards are the only other place a tab is drawn.
 */
export const TabList = () => {
  const display = useContext(DisplayContext);

  return (
    <section class="space-y-3" aria-labelledby="tabs-heading">
      <h2 id="tabs-heading" class={SECTION_HEADING}>All tabs</h2>
      <Errored fallback={(error, reset) => <RegionFailure error={error()} retry={reset} />}>
        <Loading fallback={<RegionPending label="Loading tabs" />}>
          <Show when={display.tabs().length > 0} fallback={<p class={EMPTY_PANEL}>No tabs configured.</p>}>
            <ul class="divide-y divide-hairline overflow-hidden rounded-panel bg-surface">
              <For each={display.tabs()} keyed={tab => tab.tab_id}>
                {tab => (
                  <li class="flex items-center gap-4 px-4 py-2.5">
                    <span class="w-56 shrink-0 truncate text-sm font-medium text-slate-900 dark:text-slate-100">{tab().name}</span>
                    <span class="min-w-0 flex-1 truncate text-xs text-slate-500 dark:text-slate-500">{tab().url ?? "Camera"}</span>
                    <button
                      type="button"
                      onClick={() => void display.change(async () => showTab(tab().tab_id), ["status"])}
                      disabled={display.status().current_tab_id === tab().tab_id}
                      aria-label={`Show ${tab().name} on screen`}
                      title="Show on screen until rotation resumes"
                      class={ICON_BUTTON}
                    >
                      <FiPlay size={14} aria-hidden="true" />
                    </button>
                    <button
                      type="button"
                      onClick={() => void display.change(async () => deleteTab(tab().tab_id), ["tabs", "playlistTabs", "playlists"])}
                      aria-label={`Delete ${tab().name}`}
                      title="Delete this tab"
                      class={DANGER_ICON_BUTTON}
                    >
                      <FiTrash2 size={14} aria-hidden="true" />
                    </button>
                  </li>
                )}
              </For>
            </ul>
          </Show>
        </Loading>
      </Errored>
    </section>
  );
};
