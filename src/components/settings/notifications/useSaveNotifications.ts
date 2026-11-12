import { useRef } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { api, type Notifications, type Settings } from "../../../api";
import { SETTINGS_KEYS } from "../../../queries/settings";
import { fail } from "../../../state/store";

/** Saves a change to Settings → Notifications at once. The page shows it before Rust answers, and each change
 *  builds on the latest one, so quick ticks in a row all stick. */
export function useSaveNotifications() {
  const client = useQueryClient();
  const latest = useRef(0);
  const put = (notifications: Notifications) =>
    client.setQueryData<Settings>(SETTINGS_KEYS.settings, (current) => current && { ...current, notifications });

  return (patch: Partial<Notifications>) => {
    const current = client.getQueryData<Settings>(SETTINGS_KEYS.settings)?.notifications;
    if (!current) return;
    const next = { ...current, ...patch };
    const request = ++latest.current;
    put(next);
    // An older answer would undo a newer tick on screen until its own answer came.
    void api.notificationsSave(next).then(
      (saved) => request === latest.current && put(saved),
      (e: unknown) => {
        if (request === latest.current) put(current);
        fail(e, "Could not save the notification settings");
      },
    );
  };
}
