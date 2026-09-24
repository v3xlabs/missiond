import { Show } from "solid-js";

import type { Alert } from "./feed";
import { MeetingIcon } from "./MeetingIcon";
import { createNow } from "./now";

const COUNTDOWN_MS = 30_000;

const edge = {
  info: "bg-sky-500",
  warning: "bg-amber-500",
  critical: "bg-red-500",
} as const;

const clock = new Intl.DateTimeFormat(undefined, { hour: "2-digit", minute: "2-digit" });
const relative = new Intl.RelativeTimeFormat(undefined, { numeric: "auto" });

const countdown = (startsAt: string, now: number) => {
  const minutes = Math.round((new Date(startsAt).getTime() - now) / 60_000);

  if (minutes <= 0 && minutes > -60) {
    return "now";
  }

  if (Math.abs(minutes) >= 60) {
    return relative.format(Math.round(minutes / 60), "hour");
  }

  return relative.format(minutes, "minute");
};

export const AlertCard = (properties: { alert: Alert; isLarge: boolean; }) => {
  const now = createNow(COUNTDOWN_MS);

  return (
    <article class={["flex w-full bg-gray-900", { "max-w-5xl": properties.isLarge }]}>
      <div class={["w-2 shrink-0", edge[properties.alert.level]]} />
      <div class={["min-w-0", properties.isLarge ? "p-12" : "p-4"]}>
        <Show when={properties.alert.starts_at}>
          {startsAt => (
            <p class={["font-medium text-gray-400", properties.isLarge ? "mb-3 text-3xl" : "mb-1 text-sm"]}>
              {clock.format(new Date(startsAt()))}
              <span class="ml-3 text-gray-500">{countdown(startsAt(), now())}</span>
            </p>
          )}
        </Show>
        <div class={["flex items-center", properties.isLarge ? "gap-5" : "gap-2"]}>
          <Show when={properties.alert.meeting}>
            {meeting => <MeetingIcon meeting={meeting()} class={properties.isLarge ? "size-14 shrink-0" : "size-5 shrink-0"} />}
          </Show>
          <h1 class={["truncate font-semibold text-gray-100", properties.isLarge ? "text-7xl" : "text-xl"]}>
            {properties.alert.title}
          </h1>
        </div>
        <Show when={properties.alert.body}>
          {body => <p class={["mt-4 text-gray-400", properties.isLarge ? "text-4xl" : "text-base"]}>{body()}</p>}
        </Show>
        <Show when={properties.alert.location}>
          {location => <p class={["text-gray-500", properties.isLarge ? "mt-4 text-3xl" : "mt-1 text-sm"]}>{location()}</p>}
        </Show>
      </div>
    </article>
  );
};
