import { Show } from "solid-js";

import { clock } from "./clock";
import { MeetingIcon } from "./MeetingIcon";
import type { Timed } from "./schedule";
import { leftPhrase } from "./schedule";

/**
 * The meeting on now is lifted off the axis. Its start is behind us, so the question it has to
 * answer is how much of it is left, which a bar answers at a glance and the axis cannot.
 */
export const RailRunning = (properties: { entry: Timed; now: number; }) => {
  const elapsed = () => {
    const length = properties.entry.endsAt - properties.entry.startsAt;

    return length > 0 ? Math.min(100, ((properties.now - properties.entry.startsAt) / length) * 100) : undefined;
  };

  return (
    <article class="shrink-0 px-[1.3em] pb-[1.2em]">
      <div class="flex items-center gap-[0.5em]">
        <Show when={properties.entry.alert.meeting}>
          {meeting => <MeetingIcon meeting={meeting()} class="size-[1.3em] shrink-0" />}
        </Show>
        <h2 class="truncate text-[1.45em] leading-tight font-semibold text-white">{properties.entry.alert.title}</h2>
      </div>
      <p class="mt-[0.25em] flex gap-[0.8em] text-[0.85em] text-gray-400 tabular-nums">
        <Show when={elapsed() !== undefined}>
          <span>{leftPhrase(properties.entry.endsAt, properties.now)}</span>
        </Show>
        <span>{`until ${clock.format(properties.entry.endsAt)}`}</span>
        <Show when={properties.entry.alert.location}>
          {location => <span class="truncate">{location()}</span>}
        </Show>
      </p>
      <Show when={elapsed() !== undefined}>
        <div class="mt-[0.7em] h-[0.2em] bg-white/15">
          <div class="h-full bg-white" style={{ width: `${elapsed() ?? 0}%` }} />
        </div>
      </Show>
    </article>
  );
};
