import { resolve } from "node:path";

import solid from "@solidjs/vite-plugin";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [solid(), tailwindcss()],
  build: {
    rolldownOptions: {
      // The alert page is what the display itself loads. It is a separate entry point so it does
      // not pull in the admin interface, which would slow down the one moment where load time is
      // visible on the wall.
      input: {
        main: resolve(import.meta.dirname, "index.html"),
        notify: resolve(import.meta.dirname, "notify.html"),
      },
    },
  },
  server: {
    port: 5173,
    proxy: {
      "/api": {
        target: "http://localhost:3000",
        changeOrigin: true,
      },
    },
  },
});
