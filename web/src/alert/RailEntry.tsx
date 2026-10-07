import { For, Show } from "solid-js";

import { clock } from "./clock";
import { MeetingIcon } from "./MeetingIcon";
import type { Nearness, Placed } from "./schedule";
import { leftPhrase, nearnessOf, percentOf } from "./schedule";

type Tone = {
  surface: string;
  title: string;
  meta: string;
};

const tones: Record<Nearness, Tone> = {
  starting: {
    surface: "bg-amber-400/15",
    title: "text-white",
    meta: "text-amber-200",
  },
  soon: {
    surface: "bg-slate-800",
    title: "text-gray-100",
    meta: "text-gray-400",
  },
  later: {
    surface: "bg-slate-800",
    title: "text-gray-300",
    meta: "text-gray-500",
  },
};

export const RailEntry = (properties: { entry: Placed; from: number; spanMs: number; now: number; }) => {
  const nearness = () => nearnessOf(properties.entry.startsAt, properties.now);
  const isRunning = () => properties.entry.startsAt <= properties.now && properties.entry.endsAt > properties.now;
  const details = () => {
    const entry = properties.entry;
    const lines: string[] = [];

    if (isRunning()) {
      lines.push(leftPhrase(entry.endsAt, properties.now));
    }
    else if (nearness() === "starting") {
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

  return (
    <li
      class="rail-entry absolute"
      title={`${properties.entry.alert.title}, ${details().join(", ")}`}
      style={{
        top: `${percentOf(Math.max(properties.entry.startsAt, properties.from), properties.from, properties.spanMs)}%`,
        height: `${((Math.min(properties.entry.endsAt, properties.from + properties.spanMs) - Math.max(properties.entry.startsAt, properties.from)) / properties.spanMs) * 100}%`,
        left: `${(properties.entry.lane / properties.entry.lanes) * 100}%`,
        width: `${(1 / properties.entry.lanes) * 100}%`,
      }}
    >
      <div class={["absolute inset-y-0 right-[0.3em] left-0", tones[nearness()].surface]} data-rail-surface data-nearness={nearness()} />
      <div class="absolute inset-y-0 right-[0.3em] left-0 flex flex-col gap-[0.1em] overflow-hidden px-[0.6em] py-[0.3em]" data-rail-label>
        <div class="flex items-center gap-[0.45em]">
          <Show when={properties.entry.alert.meeting}>
            {meeting => <MeetingIcon meeting={meeting()} class="size-[1.1em] shrink-0" />}
          </Show>
          <h2 class={["truncate text-[1em] leading-tight font-semibold", tones[nearness()].title]}>{properties.entry.alert.title}</h2>
        </div>
        <p class={["rail-entry-detail flex flex-wrap gap-x-[0.7em] text-[0.8em] tabular-nums", tones[nearness()].meta]}>
          <For each={details()}>
            {detail => <span class="truncate">{detail}</span>}
          </For>
        </p>
      </div>
      <Show when={properties.entry.endsAt === properties.entry.startsAt}>
        <span class="absolute inset-x-0 top-0 h-px bg-gray-300" />
        <p class="absolute top-0 right-[0.3em] left-[0.6em] -translate-y-1/2 truncate text-[1em] text-gray-300">{properties.entry.alert.title}</p>
      </Show>
    </li>
  );
};
