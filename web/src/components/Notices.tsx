import { FiX } from "solid-icons/fi";
import { For } from "solid-js";

import { dismiss, notices } from "../api/notices";
import { ICON_BUTTON } from "./controls";

export const Notices = () => (
  <div class="fixed right-4 bottom-4 z-50 flex w-80 max-w-[calc(100vw-2rem)] flex-col gap-2">
    <For each={notices()} keyed={notice => notice.notice_id}>
      {notice => (
        <div role="alert" class="flex items-start gap-2 rounded-panel bg-surface py-2 pr-2 pl-4 shadow-lg ring-1 ring-hairline">
          <p class="min-w-0 flex-1 py-1 text-sm text-red-600 dark:text-red-400">{notice().message}</p>
          <button
            type="button"
            onClick={() => dismiss(notice().notice_id)}
            aria-label="Dismiss"
            title="Dismiss"
            class={ICON_BUTTON}
          >
            <FiX size={14} aria-hidden="true" />
          </button>
        </div>
      )}
    </For>
  </div>
);
