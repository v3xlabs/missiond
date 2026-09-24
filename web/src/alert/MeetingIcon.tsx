import { Dynamic } from "@solidjs/web";
import type { IconTypes } from "solid-icons";
import { FiVideo } from "solid-icons/fi";
import { SiGooglemeet, SiJitsi, SiWebex, SiZoom } from "solid-icons/si";

import type { Alert } from "./feed";

const icons: Record<string, IconTypes> = {
  zoom: SiZoom,
  meet: SiGooglemeet,
  jitsi: SiJitsi,
  webex: SiWebex,
};

const brand: Record<string, string> = {
  zoom: "text-[#0b5cff]",
  meet: "text-[#00832d]",
  jitsi: "text-[#1d76ba]",
  webex: "text-[#00bceb]",
};

// The icon package types its props against Solid 1's JSX, which has no `class`, so the size and
// colour sit on a wrapper the icon fills.
export const MeetingIcon = (properties: { meeting: NonNullable<Alert["meeting"]>; class: string; }) => (
  <span
    role="img"
    aria-label={properties.meeting.provider ?? "meeting"}
    class={["inline-flex", brand[properties.meeting.provider ?? ""] ?? "text-gray-500", properties.class]}
  >
    <Dynamic component={icons[properties.meeting.provider ?? ""] ?? FiVideo} size="100%" aria-hidden="true" />
  </span>
);
