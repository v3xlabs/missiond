import { createDisplay, DisplayContext } from "../app/display";
import { Header } from "../components/Header";
import { Notices } from "../components/Notices";
import { NowPlaying } from "../components/NowPlaying";
import { PlaylistList } from "../sections/PlaylistList";
import { StingerList } from "../sections/StingerList";
import { TabList } from "../sections/TabList";

export const App = () => {
  // Built once here: a call written into the prop would become a getter that builds anew on each read.
  const display = createDisplay();

  return (
    <DisplayContext value={display}>
      <div class="min-h-screen text-slate-900 dark:text-slate-100">
        <Header />
        <main class="mx-auto max-w-6xl space-y-8 px-6 py-8">
          <NowPlaying />
          <PlaylistList />
          <TabList />
          <StingerList />
        </main>
        <Notices />
      </div>
    </DisplayContext>
  );
};
