import { FiPlus } from "solid-icons/fi";
import { createSignal, Show, useContext } from "solid-js";

import { createPlaylist } from "../api/playlists";
import { DisplayContext } from "../app/display";
import { FIELD, FIELD_LABEL, PRIMARY_BUTTON, readText, SECONDARY_BUTTON } from "./controls";
import { Modal } from "./Modal";

export const CreatePlaylistDialog = () => {
  const display = useContext(DisplayContext);
  const [isOpen, setIsOpen] = createSignal(false);
  const [isSubmitting, setIsSubmitting] = createSignal(false);
  const [failure, setFailure] = createSignal<string | null>(null);
  let form: HTMLFormElement | undefined;

  const close = () => {
    setIsOpen(false);
    setFailure(null);
    form?.reset();
  };

  const submit = async (event: SubmitEvent & { currentTarget: HTMLFormElement; }) => {
    event.preventDefault();

    const fields = new FormData(event.currentTarget);
    const name = readText(fields, "name");

    setIsSubmitting(true);

    const result = await display.apply(async () => createPlaylist({
      playlist_id: readText(fields, "playlist_id"),
      interval: readText(fields, "interval"),
      ...(name !== "" && { name }),
    }), ["playlists"]);

    setIsSubmitting(false);

    if (result.ok) close();
    else setFailure(result.message);
  };

  return (
    <>
      <button type="button" onClick={() => setIsOpen(true)} class={[PRIMARY_BUTTON, "flex items-center gap-1.5"]}>
        <FiPlus size={14} aria-hidden="true" />
        New playlist
      </button>
      <Modal title="New playlist" isOpen={isOpen()} onClose={close}>
        <form
          ref={(element) => {
            form = element;
          }}
          class="space-y-4"
          onSubmit={event => void submit(event)}
        >
          <div class="space-y-1">
            <label for="playlist-id" class={FIELD_LABEL}>Playlist id</label>
            <input
              id="playlist-id"
              name="playlist_id"
              type="text"
              required
              autofocus
              placeholder="lobby"
              class={FIELD}
            />
          </div>
          <div class="space-y-1">
            <label for="playlist-name" class={FIELD_LABEL}>Name (optional)</label>
            <input
              id="playlist-name"
              name="name"
              type="text"
              placeholder="Lobby"
              class={FIELD}
            />
          </div>
          <div class="space-y-1">
            <label for="playlist-interval" class={FIELD_LABEL}>Interval</label>
            <input
              id="playlist-interval"
              name="interval"
              type="text"
              required
              value="1m"
              aria-describedby="playlist-interval-hint"
              class={FIELD}
            />
            <p id="playlist-interval-hint" class="text-xs text-slate-500 dark:text-slate-500">A duration such as 30s, 5m or 1h.</p>
          </div>
          <Show when={failure()}>
            {message => <p class="text-sm text-red-600 dark:text-red-400" role="alert">{message()}</p>}
          </Show>
          <div class="flex justify-end gap-2">
            <button type="button" onClick={close} class={SECONDARY_BUTTON}>Cancel</button>
            <button type="submit" disabled={isSubmitting()} class={PRIMARY_BUTTON}>
              {isSubmitting() ? "Creating..." : "Create"}
            </button>
          </div>
        </form>
      </Modal>
    </>
  );
};
