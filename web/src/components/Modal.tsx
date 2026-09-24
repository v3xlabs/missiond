import type { JSX } from "@solidjs/web";
import { createEffect, createUniqueId } from "solid-js";

// A native modal `dialog` sits in the browser's top layer, which escapes the clipping of the
// scrolling tab strip, and it brings Escape, focus trapping and `autofocus` with it.
export const Modal = (properties: {
  title: string;
  isOpen: boolean;
  onClose: () => void;
  children: JSX.Element;
}) => {
  const titleId = createUniqueId();
  let dialog: HTMLDialogElement | undefined;

  createEffect(
    () => properties.isOpen,
    (isOpen) => {
      if (isOpen) dialog?.showModal();
      else dialog?.close();
    },
  );

  return (
    <dialog
      ref={(element) => {
        dialog = element;
      }}
      aria-labelledby={titleId}
      onClose={() => properties.onClose()}
      // The content fills the dialog box, so a click that lands on the dialog itself hit the backdrop.
      onClick={(event) => {
        if (event.target === event.currentTarget) properties.onClose();
      }}
      class="m-auto w-[min(32rem,calc(100vw-2rem))] rounded-panel bg-surface p-0 text-sm text-slate-900 shadow-xl ring-1 ring-hairline backdrop:bg-slate-950/40 dark:text-slate-100"
    >
      <div class="space-y-4 p-5">
        <h2 id={titleId} class="text-base font-semibold">{properties.title}</h2>
        {properties.children}
      </div>
    </dialog>
  );
};
