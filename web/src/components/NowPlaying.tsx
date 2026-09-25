import { FiChevronLeft, FiChevronRight, FiMonitor, FiPause, FiPlay } from "solid-icons/fi";
import { TbOutlineLayoutSidebarRightCollapse, TbOutlineLayoutSidebarRightExpand } from "solid-icons/tb";
import { Errored, Loading, Show, useContext } from "solid-js";

import type { PlaybackAction } from "../api/display";
import { playback, setScreenPower, setSidebarMode } from "../api/display";
import { DisplayContext } from "../app/display";
import { SECONDARY_BUTTON, SECTION_HEADING } from "./controls";
import { RegionFailure, RegionPending } from "./Region";
import { ScreenPanel } from "./ScreenPanel";

const SEGMENT = "flex size-7 items-center justify-center rounded-[calc(var(--radius-control)-2px)] text-slate-600 hover:bg-surface hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100";

const TEXT_SEGMENT = "flex h-7 items-center rounded-[calc(var(--radius-control)-2px)] px-2 text-xs font-medium text-slate-600 hover:bg-surface hover:text-slate-900 aria-pressed:bg-surface aria-pressed:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100 dark:aria-pressed:text-slate-100";

const Status = () => {
  const display = useContext(DisplayContext);

  const step = (action: PlaybackAction) => void display.change(async () => playback(action), ["status"]);

  const tabName = () => {
    const tabId = display.status().current_tab_id;

    return tabId === undefined ? undefined : display.tabs().find(tab => tab.tab_id === tabId)?.name ?? tabId;
  };

  const playlistName = () => {
    const playlistId = display.status().current_playlist_id;

    return display.playlists().find(playlist => playlist.playlist_id === playlistId)?.name ?? "No playlist";
  };

  return (
    <div class="flex flex-wrap items-center justify-between gap-4 px-4 py-3">
      <div class="min-w-0 space-y-0.5">
        <p class="truncate text-base font-semibold text-slate-900 dark:text-slate-100">{tabName() ?? "Nothing on screen"}</p>
        <p class="text-xs text-slate-500 dark:text-slate-400">
          {`${playlistName()}, ${display.status().auto_rotate ? "rotating" : "rotation paused"}`}
        </p>
        <Show when={display.status().config_read_only}>
          <p class="text-xs text-amber-700 dark:text-amber-400">
            The config directory is managed elsewhere. Changes apply now and are lost on restart.
          </p>
        </Show>
      </div>
      <div class="flex shrink-0 items-center gap-2">
        <div role="group" aria-label="Playback" class="inline-flex rounded-control bg-raised p-0.5">
          <button
            type="button"
            onClick={() => step("previous")}
            aria-label="Previous tab"
            title="Previous tab"
            class={SEGMENT}
          >
            <FiChevronLeft size={16} aria-hidden="true" />
          </button>
          <Show
            when={display.status().auto_rotate}
            fallback={(
              <button
                type="button"
                onClick={() => step("resume")}
                aria-label="Resume rotation"
                title="Resume rotation"
                class={SEGMENT}
              >
                <FiPlay size={14} aria-hidden="true" />
              </button>
            )}
          >
            <button
              type="button"
              onClick={() => step("pause")}
              aria-label="Pause rotation"
              title="Pause rotation"
              class={SEGMENT}
            >
              <FiPause size={14} aria-hidden="true" />
            </button>
          </Show>
          <button
            type="button"
            onClick={() => step("next")}
            aria-label="Next tab"
            title="Next tab"
            class={SEGMENT}
          >
            <FiChevronRight size={16} aria-hidden="true" />
          </button>
        </div>
        <div role="group" aria-label="Sidebar" class="inline-flex rounded-control bg-raised p-0.5">
          <Show
            when={display.status().sidebar.open}
            fallback={(
              <button
                type="button"
                onClick={() => void display.change(async () => setSidebarMode("open"), ["status"])}
                aria-label="Expand the sidebar"
                title="Expand the sidebar"
                class={SEGMENT}
              >
                <TbOutlineLayoutSidebarRightExpand size={16} aria-hidden="true" />
              </button>
            )}
          >
            <button
              type="button"
              onClick={() => void display.change(async () => setSidebarMode("closed"), ["status"])}
              aria-label="Collapse the sidebar"
              title="Collapse the sidebar"
              class={SEGMENT}
            >
              <TbOutlineLayoutSidebarRightCollapse size={16} aria-hidden="true" />
            </button>
          </Show>
          <button
            type="button"
            onClick={() => void display.change(async () => setSidebarMode("auto"), ["status"])}
            aria-pressed={display.status().sidebar.mode === "auto" ? "true" : "false"}
            title="Let notifications and the calendar open and close the sidebar"
            class={TEXT_SEGMENT}
          >
            Auto
          </button>
        </div>
        <button
          type="button"
          aria-pressed={display.status().screen_on ? "true" : "false"}
          onClick={() => void display.change(async () => setScreenPower(!display.status().screen_on), ["status"])}
          title={display.status().screen_on ? "Turn the screen off" : "Turn the screen on"}
          class={SECONDARY_BUTTON}
        >
          <span class={display.status().screen_on ? "text-emerald-600 dark:text-emerald-400" : "text-slate-400 dark:text-slate-500"}>
            <FiMonitor size={14} aria-hidden="true" />
          </span>
          {display.status().screen_on ? "Screen on" : "Screen off"}
        </button>
      </div>
    </div>
  );
};

export const NowPlaying = () => (
  <section class="space-y-3" aria-labelledby="now-playing-heading">
    <h2 id="now-playing-heading" class={SECTION_HEADING}>Now playing</h2>
    <div class="divide-y divide-hairline rounded-panel bg-surface">
      <Errored fallback={(error, reset) => <div class="px-4"><RegionFailure error={error()} retry={reset} /></div>}>
        <Loading fallback={<div class="px-4"><RegionPending label="Connecting to the display" /></div>}>
          <Status />
        </Loading>
      </Errored>
      <ScreenPanel />
    </div>
  </section>
);
