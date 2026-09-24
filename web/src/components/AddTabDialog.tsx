import { FiPlus } from "solid-icons/fi";
import { createSignal, Errored, For, Loading, Show, useContext } from "solid-js";

import { addTabToPlaylist } from "../api/playlists";
import { upsertTab } from "../api/tabs";
import { DisplayContext } from "../app/display";
import { FIELD, FIELD_LABEL, PRIMARY_BUTTON, readText, SECONDARY_BUTTON } from "./controls";
import { Modal } from "./Modal";

export const AddTabDialog = (properties: { playlistId: string; playlistName: string; }) => {
  const display = useContext(DisplayContext);
  const [isOpen, setIsOpen] = createSignal(false);
  const [isSubmitting, setIsSubmitting] = createSignal(false);
  const [failure, setFailure] = createSignal<string | null>(null);
  let form: HTMLFormElement | undefined;

  const unused = () => {
    const inPlaylist = display.playlistTabs().get(properties.playlistId) ?? [];

    return display.tabs().filter(tab => inPlaylist.every(member => member.tab_id !== tab.tab_id));
  };

  const close = () => {
    setIsOpen(false);
    setFailure(null);
    form?.reset();
  };

  const add = async (tabId: string) => {
    const result = await display.apply(
      async () => addTabToPlaylist(properties.playlistId, tabId),
      ["playlistTabs", "playlists"],
    );

    if (result.ok) close();
    else setFailure(result.message);
  };

  const submit = async (event: SubmitEvent & { currentTarget: HTMLFormElement; }) => {
    event.preventDefault();

    const fields = new FormData(event.currentTarget);
    const tabId = readText(fields, "tab_id");
    const name = readText(fields, "name");

    setIsSubmitting(true);

    const result = await display.apply(
      async () => upsertTab(tabId, { url: readText(fields, "url"), ...(name !== "" && { name }) }),
      ["tabs", "playlistTabs"],
    );

    if (result.ok) await add(tabId);
    else setFailure(result.message);

    setIsSubmitting(false);
  };

  return (
    <>
      <button type="button" onClick={() => setIsOpen(true)} class={SECONDARY_BUTTON}>
        <FiPlus size={14} aria-hidden="true" />
        Add tab
      </button>
      <Modal title={`Add a tab to ${properties.playlistName}`} isOpen={isOpen()} onClose={close}>
        <Errored fallback={null}>
          <Loading fallback={null}>
            <Show when={unused().length > 0}>
              <div class="space-y-1.5">
                <p class={FIELD_LABEL}>Existing tabs</p>
                <div class="flex flex-wrap gap-2">
                  <For each={unused()} keyed={tab => tab.tab_id}>
                    {tab => (
                      <button type="button" onClick={() => void add(tab().tab_id)} class={SECONDARY_BUTTON}>
                        {tab().name}
                      </button>
                    )}
                  </For>
                </div>
              </div>
            </Show>
          </Loading>
        </Errored>
        <form
          ref={(element) => {
            form = element;
          }}
          class="space-y-4"
          onSubmit={event => void submit(event)}
        >
          <div class="space-y-1">
            <label for={`${properties.playlistId}-tab-id`} class={FIELD_LABEL}>Tab id</label>
            <input
              id={`${properties.playlistId}-tab-id`}
              name="tab_id"
              type="text"
              required
              placeholder="overview"
              class={FIELD}
            />
          </div>
          <div class="space-y-1">
            <label for={`${properties.playlistId}-tab-name`} class={FIELD_LABEL}>Name (optional)</label>
            <input
              id={`${properties.playlistId}-tab-name`}
              name="name"
              type="text"
              placeholder="Overview"
              class={FIELD}
            />
          </div>
          <div class="space-y-1">
            <label for={`${properties.playlistId}-tab-url`} class={FIELD_LABEL}>URL</label>
            <input
              id={`${properties.playlistId}-tab-url`}
              name="url"
              type="text"
              required
              placeholder="https://grafana.example.com/d/overview?kiosk"
              class={FIELD}
            />
          </div>
          <Show when={failure()}>
            {message => <p class="text-sm text-red-600 dark:text-red-400" role="alert">{message()}</p>}
          </Show>
          <div class="flex justify-end gap-2">
            <button type="button" onClick={close} class={SECONDARY_BUTTON}>Cancel</button>
            <button type="submit" disabled={isSubmitting()} class={PRIMARY_BUTTON}>
              {isSubmitting() ? "Adding..." : "Add tab"}
            </button>
          </div>
        </form>
      </Modal>
    </>
  );
};
