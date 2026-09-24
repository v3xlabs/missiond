import { Errored, For, Loading, Show, useContext } from "solid-js";

import { DisplayContext } from "../app/display";
import { EMPTY_PANEL, SECTION_HEADING } from "../components/controls";
import { CreatePlaylistDialog } from "../components/CreatePlaylistDialog";
import { PlaylistCard } from "../components/PlaylistCard";
import { RegionFailure, RegionPending } from "../components/Region";

export const PlaylistList = () => {
  const display = useContext(DisplayContext);

  return (
    <section class="space-y-3" aria-labelledby="playlists-heading">
      <div class="flex items-center justify-between gap-4">
        <h2 id="playlists-heading" class={SECTION_HEADING}>Playlists</h2>
        <CreatePlaylistDialog />
      </div>
      <Errored fallback={(error, reset) => <RegionFailure error={error()} retry={reset} />}>
        <Loading fallback={<RegionPending label="Loading playlists" />}>
          <Show when={display.playlists().length > 0} fallback={<p class={EMPTY_PANEL}>No playlists configured.</p>}>
            <div class="space-y-4">
              <For each={display.playlists()} keyed={playlist => playlist.playlist_id}>
                {playlist => <PlaylistCard playlist={playlist()} />}
              </For>
            </div>
          </Show>
        </Loading>
      </Errored>
    </section>
  );
};
