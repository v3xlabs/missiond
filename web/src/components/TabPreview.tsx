import { createSignal, Show } from "solid-js";

import { baseUrl } from "../api/api";

/**
 * A tab has no frame until its page has painted at least once, so a tab the rotation has not
 * reached yet has nothing to show. Saying so beats an empty box the reader reads as broken.
 */
export const TabPreview = (properties: { tabId: string; }) => {
  const [isAvailable, setIsAvailable] = createSignal(true);

  return (
    <>
      {/* A multipart stream never finishes loading, so `load` is no signal that it works. Only
          the failure is observable, and that is what the message reports. The URL is built from
          the one this page was served from, so the stream follows the host the reader actually
          typed rather than whatever the daemon believes it is called. */}
      <img
        src={new URL(`preview_live/${encodeURIComponent(properties.tabId)}`, baseUrl).href}
        alt=""
        draggable={false}
        onError={() => setIsAvailable(false)}
        onLoad={() => setIsAvailable(true)}
        class="size-full object-cover"
      />
      <Show when={!isAvailable()}>
        <span class="absolute inset-0 flex items-center justify-center px-3 text-center text-xs text-slate-500 dark:text-slate-400">
          Not rendered yet
        </span>
      </Show>
    </>
  );
};
