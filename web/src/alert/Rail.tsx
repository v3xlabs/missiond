import { createMemo, For, Show } from "solid-js";

import { clock } from "./clock";
import type { Alert } from "./feed";
import { createNow } from "./now";
import { RailEntry } from "./RailEntry";
import { RailNotice } from "./RailNotice";
import { RailRunning } from "./RailRunning";
import { percentOf, placed, spanOf, ticksOf, timedOf } from "./schedule";

/** Near enough for "in 4 min" to be true, far enough to leave the display alone. */
const TICK_MS = 15_000;

const today = new Intl.DateTimeFormat(undefined, { weekday: "long", day: "numeric", month: "long" });
const otherDay = new Intl.DateTimeFormat(undefined, { weekday: "long" });

export const Rail = (properties: { alerts: readonly Alert[]; isWall: boolean; }) => {
  const now = createNow(TICK_MS);
  const timed = createMemo(() => timedOf(properties.alerts));
  const ahead = createMemo(() => timed().filter(entry => entry.startsAt > now()));
  const spanMs = createMemo(() => spanOf(ahead(), now()));

  return (
    <main class={["flex h-screen w-screen flex-col bg-black", properties.isWall ? "text-[24px]" : "text-[15px]"]}>
      <header class="flex shrink-0 items-baseline justify-between px-[1.3em] pt-[1.1em] pb-[1em]">
        <p class="text-[0.8em] font-medium tracking-[0.18em] text-gray-500 uppercase">
          {today.format(now())}
        </p>
        <time class="text-[1.1em] font-medium text-gray-300 tabular-nums">{clock.format(now())}</time>
      </header>

      <Show when={properties.alerts.some(alert => !alert.starts_at)}>
        <ul class="shrink-0 px-[0.6em] pb-[0.6em]">
          <For each={properties.alerts.filter(alert => !alert.starts_at)} keyed={alert => alert.notification_id}>
            {alert => <RailNotice alert={alert()} />}
          </For>
        </ul>
      </Show>

      <For each={timed().filter(entry => entry.startsAt <= now())} keyed={entry => entry.alert.notification_id}>
        {entry => <RailRunning entry={entry()} now={now()} />}
      </For>

      <Show
        when={ahead().length > 0}
        fallback={(
          <p class="flex flex-1 items-center justify-center text-[1.2em] text-gray-600">
            {timed().length > 0 ? "Nothing else today" : "Nothing scheduled"}
          </p>
        )}
      >
        <div class="relative min-h-0 flex-1 px-[0.7em] pb-[0.7em]">
          <time class="absolute top-0 left-[0.7em] text-[0.78em] font-semibold text-white tabular-nums">
            {clock.format(now())}
          </time>

          <ol>
            <For each={ticksOf(now(), spanMs())}>
              {at => (
                <li
                  class="absolute inset-x-[0.7em] flex items-center gap-[0.6em]"
                  style={{ top: `${percentOf(at, now(), spanMs())}%` }}
                >
                  <time class="w-[4.6em] shrink-0 text-[0.78em] whitespace-nowrap text-gray-600 tabular-nums">
                    {new Date(at).getHours() === 0 ? otherDay.format(at) : clock.format(at)}
                  </time>
                  <span class="h-px flex-1 bg-white/8" />
                </li>
              )}
            </For>
          </ol>

          {/* The gutter is the axis, so the entries hang to the right of every hour label. */}
          <ol class="absolute top-0 right-[0.7em] bottom-[0.7em] left-[5.9em]">
            <For each={placed(ahead())} keyed={entry => entry.alert.notification_id}>
              {entry => (
                <RailEntry
                  entry={entry()}
                  from={now()}
                  spanMs={spanMs()}
                  now={now()}
                />
              )}
            </For>
          </ol>
        </div>
      </Show>
    </main>
  );
};
