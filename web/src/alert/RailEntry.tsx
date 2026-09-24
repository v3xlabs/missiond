import { For, Show } from "solid-js";

import { clock } from "./clock";
import { MeetingIcon } from "./MeetingIcon";
import type { Nearness, Placed } from "./schedule";
import { nearnessOf, percentOf } from "./schedule";

type Tone = {
  surface: string;
  title: string;
  meta: string;
};

/** Distance is drawn by the axis, so a tone only has to say how soon, not how far. */
const tones: Record<Nearness, Tone> = {
  starting: {
    surface: "bg-amber-400/15",
    title: "text-white",
    meta: "text-amber-200",
  },
  soon: {
    surface: "bg-white/8",
    title: "text-gray-100",
    meta: "text-gray-400",
  },
  later: {
    surface: "bg-white/5",
    title: "text-gray-300",
    meta: "text-gray-500",
  },
};

export const RailEntry = (properties: { entry: Placed; from: number; spanMs: number; now: number; }) => {
  const nearness = () => nearnessOf(properties.entry.startsAt, properties.now);

  const details = () => {
    const entry = properties.entry;
    const lines: string[] = [];

    if (nearness() === "starting") {
      lines.push(`in ${Math.max(1, Math.round((entry.startsAt - properties.now) / 60_000))} min`);
    }

    lines.push(
      entry.endsAt > entry.startsAt
        ? `${clock.format(entry.startsAt)} to ${clock.format(entry.endsAt)}`
        : clock.format(entry.startsAt),
    );

    if (entry.alert.location) {
      lines.push(entry.alert.location);
    }

    return lines;
  };

  const top = () => percentOf(properties.entry.startsAt, properties.from, properties.spanMs);

  return (
    <li
      class="rail-entry absolute"
      style={{
        "top": `${top()}%`,
        "height": `${percentOf(properties.entry.endsAt, properties.from, properties.spanMs) - top()}%`,
        "left": `${(properties.entry.lane / properties.entry.lanes) * 100}%`,
        "width": `${(1 / properties.entry.lanes) * 100}%`,
        // A quarter of an hour is four pixels of honest height, and no title fits in four pixels.
        "min-height": "2.6em",
      }}
    >
      <div class={["mr-[0.3em] mb-[0.15em] h-full overflow-hidden px-[0.6em] py-[0.3em]", tones[nearness()].surface]}>
        <div class="flex items-center gap-[0.45em]">
          <Show when={properties.entry.alert.meeting}>
            {meeting => <MeetingIcon meeting={meeting()} class="size-[1.1em] shrink-0" />}
          </Show>
          <h2 class={["truncate text-[1.05em] font-semibold", tones[nearness()].title]}>{properties.entry.alert.title}</h2>
        </div>
        <p class={["rail-entry-detail mt-[0.1em] gap-[0.7em] text-[0.8em] tabular-nums", tones[nearness()].meta]}>
          <For each={details()}>
            {detail => <span class="truncate">{detail}</span>}
          </For>
        </p>
      </div>
    </li>
  );
};
