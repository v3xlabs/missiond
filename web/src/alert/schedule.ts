import type { Alert } from "./feed";

/** An alert that names a time, which is what the axis can place. */
export type Timed = {
  alert: Alert;
  startsAt: number;
  endsAt: number;
};

/** A place on the axis. Meetings that overlap share the width, one lane each. */
export type Placed = Timed & {
  lane: number;
  lanes: number;
};

export type Nearness = "starting" | "soon" | "later";

const MINUTE_MS = 60_000;
const HOUR_MS = 60 * MINUTE_MS;
const STARTING_MS = 5 * MINUTE_MS;
const SOON_MS = HOUR_MS;

/** Short enough to read, long enough that one short meeting does not fill the rail on its own. */
const SHORTEST_SPAN_MS = 2 * HOUR_MS;

export const timedOf = (alerts: readonly Alert[]): Timed[] =>
  alerts
    .flatMap((alert) => {
      if (!alert.starts_at) {
        return [];
      }

      const startsAt = new Date(alert.starts_at).getTime();

      // An alert with no end is a moment rather than a span, and takes the height of one line.
      return [{
        alert,
        startsAt,
        endsAt: alert.ends_at ? new Date(alert.ends_at).getTime() : startsAt,
      }];
    })
    .toSorted((one, other) => one.startsAt - other.startsAt);

/**
 * Lanes are counted per run of meetings that overlap, so two at one time each take half the width
 * and a meeting standing on its own keeps all of it.
 */
export const placed = (entries: readonly Timed[]): Placed[] => {
  const out: Placed[] = [];
  let run: Timed[] = [];
  let runEnds = 0;

  const flush = () => {
    if (run.length === 0) {
      return;
    }

    const laneEnds: number[] = [];
    const assigned = run.map((entry) => {
      const free = laneEnds.findIndex(end => end <= entry.startsAt);
      const lane = free === -1 ? laneEnds.length : free;

      laneEnds[lane] = entry.endsAt;

      return { entry, lane };
    });

    for (const { entry, lane } of assigned) {
      out.push({ ...entry, lane, lanes: laneEnds.length });
    }

    run = [];
    runEnds = 0;
  };

  for (const entry of entries) {
    if (entry.startsAt >= runEnds) {
      flush();
    }

    run.push(entry);
    runEnds = Math.max(runEnds, entry.endsAt);
  }

  flush();

  return out;
};

export const spanOf = (entries: readonly Timed[], now: number) =>
  Math.max(SHORTEST_SPAN_MS, ...entries.map(entry => entry.endsAt - now));

export const percentOf = (at: number, from: number, spanMs: number) =>
  ((at - from) / spanMs) * 100;

/**
 * Hour marks, thinned as the axis covers more of the day, so two labels never sit on each other.
 * The first mark of the day is dropped when it lands under the current time, which is written at
 * the top of the same gutter.
 */
export const ticksOf = (from: number, spanMs: number): number[] => {
  const step = stepHoursOf(spanMs);
  const clear = from + spanMs * 0.04;
  const ticks: number[] = [];
  const hour = new Date(from);

  hour.setMinutes(0, 0, 0);

  for (let at = hour.getTime(); at < from + spanMs; at += HOUR_MS) {
    if (at >= clear && new Date(at).getHours() % step === 0) {
      ticks.push(at);
    }
  }

  return ticks;
};

const stepHoursOf = (spanMs: number) => {
  if (spanMs <= 8 * HOUR_MS) {
    return 1;
  }

  return spanMs <= 16 * HOUR_MS ? 2 : 3;
};

export const nearnessOf = (startsAt: number, now: number): Nearness => {
  const untilMs = startsAt - now;

  if (untilMs <= STARTING_MS) {
    return "starting";
  }

  return untilMs <= SOON_MS ? "soon" : "later";
};

/** What is left of a meeting that is already running, which is the one number the bar cannot give. */
export const leftPhrase = (endsAt: number, now: number) => {
  const minutes = Math.max(0, Math.round((endsAt - now) / MINUTE_MS));

  if (minutes < 60) {
    return `${minutes} min left`;
  }

  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;

  return rest === 0 ? `${hours} h left` : `${hours} h ${rest} min left`;
};
