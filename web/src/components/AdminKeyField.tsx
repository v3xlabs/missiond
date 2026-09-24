import { FiLock, FiUnlock } from "solid-icons/fi";
import { refresh, Show, useContext } from "solid-js";

import { adminKey, setAdminKey } from "../api/auth";
import type { components } from "../api/schema.gen";
import { DisplayContext } from "../app/display";

type Access = components["schemas"]["Access"];

type Look = {
  title: string;
  lock: string;
};

const describe: Record<Access, Look> = {
  admin: { title: "Key accepted", lock: "text-emerald-600 dark:text-emerald-400" },
  control: {
    title: "Control key: the screen can be driven, configuration edits will be refused",
    lock: "text-amber-600 dark:text-amber-400",
  },
  none: { title: "Changes are refused until this key matches", lock: "text-red-600 dark:text-red-400" },
};

export const AdminKeyField = () => {
  const display = useContext(DisplayContext);

  return (
    <Show
      when={display.status().requires_auth}
      fallback={(
        <span
          class="flex items-center gap-1.5 text-xs text-amber-700 dark:text-amber-400"
          title="No admin key is configured on the daemon, so anything that can reach this port can change the display."
        >
          <FiUnlock size={14} aria-hidden="true" />
          No admin key
        </span>
      )}
    >
      <label
        class="flex items-center gap-2 rounded-control bg-raised px-2.5 py-1 focus-within:ring-2 focus-within:ring-slate-400 dark:focus-within:ring-slate-500"
        title={describe[display.status().access].title}
      >
        <span class={describe[display.status().access].lock}>
          <FiLock size={14} aria-hidden="true" />
        </span>
        <span class="sr-only">Admin key</span>
        <input
          type="password"
          autocomplete="off"
          value={adminKey()}
          onInput={(event) => {
            setAdminKey(event.currentTarget.value);
            // The daemon reports what the key it just saw allows, so refetching status is what
            // changes the lock's colour.
            void refresh(display.status);
          }}
          placeholder="Admin key"
          aria-invalid={display.status().access === "none" ? "true" : undefined}
          class="w-36 bg-transparent text-sm text-slate-900 outline-none placeholder:text-slate-400 dark:text-slate-100 dark:placeholder:text-slate-500"
        />
      </label>
    </Show>
  );
};
