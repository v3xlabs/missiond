import { useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";

import { baseUrl } from "../api/api";
import type { components } from "../api/schema.gen";

type DeviceStatus = components["schemas"]["DeviceStatus"];

type StateFrame = Pick<
  DeviceStatus,
  "auto_rotate" | "brightness" | "current_playlist_id" | "current_tab_id" | "screen_on"
>;

export const useDisplayEvents = () => {
  const client = useQueryClient();

  useEffect(() => {
    const source = new EventSource(new URL("events", baseUrl));

    // The frame also carries the access of this connection, which an EventSource opens without
    // the admin key. Only the display fields are taken, so the status keeps the access the
    // keyed status request reported.
    source.addEventListener("state", (event) => {
      const frame = JSON.parse(event.data) as StateFrame;

      client.setQueryData(["status"], (status: DeviceStatus | undefined) => status && {
        ...status,
        auto_rotate: frame.auto_rotate,
        brightness: frame.brightness,
        current_playlist_id: frame.current_playlist_id,
        current_tab_id: frame.current_tab_id,
        screen_on: frame.screen_on,
      });
    });

    return () => source.close();
  }, [client]);
};
