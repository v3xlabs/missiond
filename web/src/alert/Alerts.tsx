import { createMemo, Match, Show, Switch } from "solid-js";

import { AlertCard } from "./AlertCard";
import type { Alert } from "./feed";
import { createAlerts } from "./feed";
import { Rail } from "./Rail";

export type Presentation = "takeover" | "sidebar" | "toast" | "agenda";

/** The agenda is the rail's content at the size of a wall, so it has no mode of its own. */
const shows = (presentation: Presentation, alert: Alert) => {
  if (presentation === "agenda") {
    return alert.mode === "sidebar";
  }

  return alert.mode === presentation;
};

export const Alerts = (properties: { presentation: Presentation; }) => {
  const feed = createAlerts();
  const alerts = createMemo(() => feed().filter(alert => shows(properties.presentation, alert)));

  return (
    <Switch fallback={<main class="h-screen w-screen bg-gray-950" />}>
      <Match when={properties.presentation === "sidebar" || properties.presentation === "agenda"}>
        <Rail alerts={alerts()} isWall={properties.presentation === "agenda"} />
      </Match>
      {/* The card fills the window, which the daemon has already sized for one toast. */}
      <Match when={properties.presentation === "toast" && alerts().at(-1)}>
        {newest => (
          <main class="flex h-screen w-screen bg-gray-950">
            <AlertCard alert={newest()} isLarge={false} />
          </main>
        )}
      </Match>
      <Match when={alerts().at(-1)}>
        {newest => (
          <main class="flex h-screen w-screen flex-col items-center justify-center bg-gray-950 p-16">
            <AlertCard alert={newest()} isLarge />
            <Show when={alerts().length > 1}>
              <p class="mt-8 text-2xl text-gray-500">{`and ${alerts().length - 1} more`}</p>
            </Show>
          </main>
        )}
      </Match>
    </Switch>
  );
};
