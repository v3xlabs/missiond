import { createMemo, For, Show } from "solid-js";

import { clock } from "./clock";
import type { Alert } from "./feed";
import { createNow } from "./now";
import { RailEntry } from "./RailEntry";
import { RailNotice } from "./RailNotice";
import { percentOf, placed, ticksOf, timedOf, WINDOW_MS } from "./schedule";

const TICK_MS = 15_000;

const today = new Intl.DateTimeFormat(undefined, { weekday: "long", day: "numeric", month: "long" });
const otherDay = new Intl.DateTimeFormat(undefined, { weekday: "short", hour: "2-digit", minute: "2-digit" });

export const Rail = (properties: { alerts: readonly Alert[]; isWall: boolean; }) => {
  const now = createNow(TICK_MS);
  const timed = createMemo(() => timedOf(properties.alerts));
  const active = createMemo(() => timed().filter(entry => entry.endsAt > now() || entry.startsAt >= now()));
  const visible = createMemo(() => active().filter(entry => entry.startsAt < now() + WINDOW_MS));
  const later = createMemo(() => active().filter(entry => entry.startsAt >= now() + WINDOW_MS));
  const timeLabel = (at: number) => (new Date(at).toDateString() === new Date(now()).toDateString()
    ? clock.format(at)
    : otherDay.format(at));

  return (
    <main class={["flex h-screen w-screen flex-col overflow-hidden bg-black", properties.isWall ? "text-[24px]" : "text-[15px]"]}>
      <header class="flex shrink-0 items-baseline justify-between gap-[0.6em] px-[1.3em] pt-[1.1em] pb-[1em]">
        <p class="text-[0.8em] font-medium tracking-[0.18em] text-gray-500 uppercase">
          {today.format(now())}
        </p>
        <time class="text-[1.1em] font-medium text-gray-300 tabular-nums">{clock.format(now())}</time>
      </header>

      <Show when={properties.alerts.some(alert => !alert.starts_at)}>
        <ul class="max-h-[25vh] shrink-0 overflow-y-auto px-[0.6em] pb-[0.6em]">
          <For each={properties.alerts.filter(alert => !alert.starts_at)} keyed={alert => alert.notification_id}>
            {alert => <RailNotice alert={alert()} />}
          </For>
        </ul>
      </Show>

      <Show
        when={active().length > 0}
        fallback={(
          <p class="flex flex-1 items-center justify-center text-[1.2em] text-gray-600">
            {timed().length > 0 ? "Nothing else today" : "Nothing scheduled"}
          </p>
        )}
      >
        <div class="relative mx-[1.3em] my-[0.6em] min-h-0 flex-1">
          <span class="absolute inset-y-0 left-[4.8em] w-px bg-gray-600" />
          <ol aria-label="Hours">
            <For each={ticksOf(now(), WINDOW_MS)}>
              {at => (
                <li
                  class="absolute inset-x-0 flex -translate-y-1/2 items-center gap-[0.6em]"
                  style={{ top: `${percentOf(at, now(), WINDOW_MS)}%` }}
                >
                  <time class="w-[5em] shrink-0 text-right text-[0.85em] whitespace-nowrap text-gray-400 tabular-nums">
                    {clock.format(at)}
                  </time>
                  <span class="h-px flex-1 bg-white/8" />
                </li>
              )}
            </For>
          </ol>
          <div class="absolute inset-x-0 top-0 flex -translate-y-1/2 items-center gap-[0.6em]">
            <time class="w-[5em] shrink-0 text-right text-[0.85em] font-semibold text-white tabular-nums">
              {clock.format(now())}
            </time>
            <span class="h-px flex-1 bg-white" />
          </div>
          <ol class="absolute inset-y-0 right-0 left-[5.5em]" aria-label="Next six hours">
            <For each={placed(visible())} keyed={entry => entry.alert.notification_id}>
              {entry => (
                <RailEntry
                  entry={entry()}
                  from={now()}
                  spanMs={WINDOW_MS}
                  now={now()}
                />
              )}
            </For>
          </ol>
        </div>

        {/* Keep the axis height unchanged when entries move into or out of the later list. */}
        <section class="flex h-[5.5em] shrink-0 gap-[0.9em] overflow-y-auto px-[1.3em] pt-[1em] pb-[0.6em]" aria-label="Later">
          <Show when={later().length > 0}>
            <h2 class="w-[3.4em] shrink-0 text-right text-[0.75em] tracking-[0.12em] text-gray-500 uppercase">Later</h2>
            <ol class="flex min-w-0 flex-1 flex-col gap-[0.5em]">
              <For each={later()} keyed={entry => entry.alert.notification_id}>
                {entry => (
                  <li class="flex flex-wrap items-baseline gap-x-[0.8em] text-[0.85em]">
                    <h3 class="min-w-0 truncate font-semibold text-gray-300">{entry().alert.title}</h3>
                    <time class="text-gray-400 tabular-nums">
                      {entry().endsAt > entry().startsAt
                        ? `${timeLabel(entry().startsAt)} to ${clock.format(entry().endsAt)}`
                        : timeLabel(entry().startsAt)}
                    </time>
                  </li>
                )}
              </For>
            </ol>
          </Show>
        </section>
      </Show>
    </main>
  );
};
