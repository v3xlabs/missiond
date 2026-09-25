import { FiPlay } from "solid-icons/fi";
import { createSignal, Errored, For, Loading, Show, useContext } from "solid-js";

import type { StingerInfo } from "../api/alerts";
import { raiseAlert } from "../api/alerts";
import { DisplayContext } from "../app/display";
import { EMPTY_PANEL, FIELD, ICON_BUTTON, SECTION_HEADING } from "../components/controls";
import { RegionFailure, RegionPending } from "../components/Region";

const CURRENT_SCREEN = "";

const StingerRow = (properties: { stinger: StingerInfo; }) => {
  const display = useContext(DisplayContext);
  const [target, setTarget] = createSignal(CURRENT_SCREEN);

  return (
    <li class="flex items-center gap-4 px-4 py-2.5">
      <span class="w-56 shrink-0 truncate text-sm font-medium text-slate-900 dark:text-slate-100">{properties.stinger.name}</span>
      <span class="min-w-0 flex-1 truncate text-xs text-slate-500 dark:text-slate-500">{properties.stinger.file}</span>
      <select
        value={target()}
        onChange={event => setTarget(event.currentTarget.value)}
        aria-label={`Where ${properties.stinger.name} leads to`}
        class={[FIELD, "w-48 shrink-0"]}
      >
        <option value={CURRENT_SCREEN}>Current screen</option>
        <For each={display.tabs()} keyed={tab => tab.tab_id}>
          {tab => <option value={tab().tab_id}>{tab().name}</option>}
        </For>
      </select>
      {/* A takeover of the tab already on screen plays the clip over it and changes nothing else. */}
      <button
        type="button"
        onClick={() => {
          const tabId = target() === CURRENT_SCREEN ? display.status().current_tab_id : target();

          void display.change(async () => raiseAlert({
            title: properties.stinger.name,
            mode: "takeover",
            stinger: properties.stinger.name,
            ...(tabId !== undefined && { tab_id: tabId }),
          }), ["status"]);
        }}
        aria-label={`Play ${properties.stinger.name}`}
        title="Play the clip, then show the chosen tab for the default alert duration"
        class={ICON_BUTTON}
      >
        <FiPlay size={14} aria-hidden="true" />
      </button>
    </li>
  );
};

/** Each clip plays through `/notify` as a takeover, the same request an automation sends. */
export const StingerList = () => {
  const display = useContext(DisplayContext);

  return (
    <section class="space-y-3" aria-labelledby="stingers-heading">
      <h2 id="stingers-heading" class={SECTION_HEADING}>Stingers</h2>
      <Errored fallback={(error, reset) => <RegionFailure error={error()} retry={reset} />}>
        <Loading fallback={<RegionPending label="Loading stingers" />}>
          <Show when={display.stingers().length > 0} fallback={<p class={EMPTY_PANEL}>No stingers configured.</p>}>
            <ul class="divide-y divide-hairline overflow-hidden rounded-panel bg-surface">
              <For each={display.stingers()} keyed={stinger => stinger.name}>
                {stinger => <StingerRow stinger={stinger()} />}
              </For>
            </ul>
          </Show>
        </Loading>
      </Errored>
    </section>
  );
};
