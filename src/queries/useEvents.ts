import { useEffect } from "react";
import { useQueryClient, type QueryKey } from "@tanstack/react-query";
import { api, type EventName } from "../api";
import { SETTINGS_KEYS } from "./settings";
import { NAV_KEY, SIDEBAR_KEY, VIEWS_KEY } from "./sidebar";
import { SYNC_KEY } from "./sync";
import { ISSUE_KEY } from "./issues";
import { COLUMNS_KEY, FILTER_OPTIONS_KEY, LIST_CONDITIONS_KEY, STATUS_STYLES_KEY, VIEW_ROWS_KEY } from "./views";

/** The query keys each Rust event makes stale. Features add their events and keys here. */
export const INVALIDATIONS: Partial<Record<EventName, QueryKey[]>> = {
  settings_changed: [SETTINGS_KEYS.settings, SETTINGS_KEYS.connection, SETTINGS_KEYS.scope],
  // Tracking a project changes the Spaces; a cache reset (issues_changed with all) empties the project list.
  // A deleted list or view moves the place, so Rust's place is asked again too. Rows depend on the lists, the
  // views, the Spaces and the status styles.
  local_changed: [SETTINGS_KEYS.projects, SETTINGS_KEYS.scope, SIDEBAR_KEY, VIEWS_KEY, NAV_KEY, VIEW_ROWS_KEY, STATUS_STYLES_KEY, LIST_CONDITIONS_KEY, FILTER_OPTIONS_KEY, ISSUE_KEY],
  issues_changed: [SETTINGS_KEYS.projects, SETTINGS_KEYS.scope, SYNC_KEY, SIDEBAR_KEY, VIEW_ROWS_KEY, COLUMNS_KEY, STATUS_STYLES_KEY, LIST_CONDITIONS_KEY, FILTER_OPTIONS_KEY, ISSUE_KEY],
  sync_state: [SYNC_KEY],
};

/** Subscribes to every event of `INVALIDATIONS` while mounted and refetches the queries it names; `nav_changed`
 *  carries the new place itself. */
export function useEvents() {
  const client = useQueryClient();
  useEffect(() => {
    const unlisten = Object.entries(INVALIDATIONS).map(([event, keys]) =>
      api.listen(event as EventName, () => {
        for (const queryKey of keys) void client.invalidateQueries({ queryKey });
      }),
    );
    unlisten.push(api.listen("nav_changed", (nav) => client.setQueryData(NAV_KEY, nav)));
    return () => unlisten.forEach((p) => void p.then((fn) => fn()).catch(() => {}));
  }, [client]);
}
