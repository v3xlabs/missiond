import { Show } from "solid-js";

import type { Alert } from "./feed";

const dot: Record<Alert["level"], string> = {
  info: "bg-sky-400",
  warning: "bg-amber-400",
  critical: "bg-red-500",
};

/** An alert with no time of its own: the feed that stopped answering, or something pushed by hand. */
export const RailNotice = (properties: { alert: Alert; }) => (
  <li class="flex items-start gap-[0.7em] px-[0.7em] py-[0.5em]">
    <span class={["mt-[0.45em] size-[0.45em] shrink-0 rounded-full", dot[properties.alert.level]]} />
    <div class="min-w-0">
      <h2 class="line-clamp-2 text-[1em] font-medium text-gray-200">{properties.alert.title}</h2>
      <Show when={properties.alert.body}>
        {body => <p class="mt-[0.2em] line-clamp-2 text-[0.85em] text-gray-500">{body()}</p>}
      </Show>
    </div>
  </li>
);
