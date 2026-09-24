import { FiRefreshCw } from "solid-icons/fi";
import { createSignal, Show } from "solid-js";

import { baseUrl } from "../api/api";
import { ICON_BUTTON, SECONDARY_BUTTON } from "./controls";

/**
 * The compositor's own output, which is what is genuinely on the panel. A tab preview only shows
 * what one page painted, so the two disagree whenever anything is drawn over the page.
 *
 * Capturing spawns a process, so this is fetched on demand rather than streamed, and nothing is
 * captured until it is opened.
 */
export const ScreenPanel = () => {
  const [takenAt, setTakenAt] = createSignal<number | undefined>();
  const [hasFailed, setHasFailed] = createSignal(false);

  const grab = () => {
    setHasFailed(false);
    setTakenAt(Date.now());
  };

  return (
    <div class="space-y-3 px-4 py-3">
      <div class="flex flex-wrap items-center gap-x-3 gap-y-1">
        <div class="min-w-0 flex-1">
          <p class="text-sm font-medium text-slate-900 dark:text-slate-100">Compositor output</p>
          <p class="text-xs text-slate-500 dark:text-slate-400">Captured from the compositor, not from a page</p>
        </div>
        <Show when={takenAt() !== undefined}>
          <button
            type="button"
            onClick={grab}
            aria-label="Capture again"
            title="Capture again"
            class={ICON_BUTTON}
          >
            <FiRefreshCw size={14} aria-hidden="true" />
          </button>
        </Show>
        <button
          type="button"
          onClick={() => (takenAt() === undefined ? grab() : setTakenAt(undefined))}
          class={SECONDARY_BUTTON}
        >
          {takenAt() === undefined ? "Capture" : "Hide"}
        </button>
      </div>
      <Show when={takenAt()}>
        {at => (
          <Show
            when={!hasFailed()}
            fallback={(
              <p class="text-sm text-slate-500 dark:text-slate-400">
                The compositor did not answer. Check that the screenshot command in display.toml is on the
                daemon&apos;s PATH.
              </p>
            )}
          >
            {/* The daemon sends no-store, but a changing query keeps a re-grab from being served out of
                the browser's in-memory image cache. */}
            <img
              src={new URL(`screen?at=${at()}`, baseUrl).href}
              alt="The display's current output"
              onError={() => setHasFailed(true)}
              class="max-h-96 w-auto rounded-control"
            />
          </Show>
        )}
      </Show>
    </div>
  );
};
