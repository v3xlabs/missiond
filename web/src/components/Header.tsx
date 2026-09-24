import { Errored, Loading, useContext } from "solid-js";

import { DisplayContext } from "../app/display";
import { AdminKeyField } from "./AdminKeyField";
import { ThemeToggle } from "./ThemeToggle";

const STATUS_TEXT = "text-sm text-slate-500 dark:text-slate-500";

export const Header = () => {
  const display = useContext(DisplayContext);

  return (
    <header>
      <div class="mx-auto flex max-w-6xl flex-wrap items-center justify-between gap-x-4 gap-y-2 px-6 py-2">
        <div class="flex min-w-0 items-center gap-3">
          <span class="shrink-0 text-sm font-semibold tracking-tight">Mission Control</span>
          <span class="text-slate-300 dark:text-slate-600" aria-hidden="true">/</span>
          <Errored fallback={<span class={STATUS_TEXT} role="alert">Daemon unreachable</span>}>
            <Loading fallback={<span class={STATUS_TEXT} role="status">Connecting</span>}>
              <span class="truncate text-sm font-medium text-slate-700 dark:text-slate-300">{display.status().device_name}</span>
            </Loading>
          </Errored>
        </div>
        <div class="flex shrink-0 items-center gap-3">
          <Errored fallback={null}>
            <Loading>
              <AdminKeyField />
            </Loading>
          </Errored>
          <ThemeToggle />
        </div>
      </div>
    </header>
  );
};
