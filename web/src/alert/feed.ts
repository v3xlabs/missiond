import type { Accessor } from "solid-js";
import { createSignal, onSettled } from "solid-js";

import type { components } from "../api/schema.gen";

export type Alert = components["schemas"]["Notification"];

export const createAlerts = (): Accessor<readonly Alert[]> => {
  const [alerts, setAlerts] = createSignal<readonly Alert[]>([]);

  onSettled(() => {
    const source = new EventSource("/api/notifications/stream");

    source.addEventListener("message", (event) => {
      try {
        setAlerts(JSON.parse(event.data));
      }
      catch {
        // A half-written frame is not worth reporting on the wall.
      }
    });

    return () => source.close();
  });

  return alerts;
};
