import { useQueryClient } from "@tanstack/react-query";
import { type FC, useState } from "react";
import { FiLock, FiUnlock } from "react-icons/fi";

import { adminKey, setAdminKey } from "../api/auth";
import type { components } from "../api/schema.gen";

type Access = components["schemas"]["Access"];

type Properties = {
  requiresAuth: boolean;
  access: Access;
};

type Look = {
  title: string;
  lock: string;
  border: string;
};

const describe: Record<Access, Look> = {
  admin: { title: "Key accepted", lock: "text-emerald-400", border: "border-gray-800" },
  control: {
    title: "Control key: the screen can be driven, configuration edits will be refused",
    lock: "text-amber-400",
    border: "border-amber-800",
  },
  none: {
    title: "Mutations will be refused until this key matches",
    lock: "text-red-400",
    border: "border-red-800",
  },
};

export const AdminKeyField: FC<Properties> = ({ requiresAuth, access }) => {
  const client = useQueryClient();
  const [key, setKey] = useState(adminKey());

  if (!requiresAuth) {
    return (
      <span
        className="flex items-center gap-1.5 bg-amber-700 px-2 py-1 text-xs text-white"
        title="No admin key is configured on the daemon, so anything that can reach this port can change the display."
      >
        <FiUnlock />
        unauthenticated
      </span>
    );
  }

  return (
    <label className="flex items-center gap-1.5" title={describe[access].title}>
      <FiLock className={describe[access].lock} />
      <input
        type="password"
        value={key}
        onChange={(event) => {
          setKey(event.target.value);
          setAdminKey(event.target.value);
          // The daemon reports whether the key it just saw was accepted, so refetching
          // status is what turns the indicator green.
          client.invalidateQueries({ queryKey: ["status"] });
        }}
        placeholder="admin key"
        className={[
          "w-40 border bg-gray-900 px-2 py-1 text-sm text-gray-100",
          describe[access].border,
        ].join(" ")}
      />
    </label>
  );
};
