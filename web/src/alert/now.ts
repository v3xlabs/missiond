import type { Accessor } from "solid-js";
import { createSignal, onSettled } from "solid-js";

/** One clock per owner, so every entry under it reads the same moment and moves on one tick. */
export const createNow = (intervalMs: number): Accessor<number> => {
  const [now, setNow] = createSignal(Date.now());

  onSettled(() => {
    const timer = setInterval(() => setNow(Date.now()), intervalMs);

    return () => clearInterval(timer);
  });

  return now;
};
