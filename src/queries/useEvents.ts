import { useEffect } from "react";
import { useQueryClient, type QueryKey } from "@tanstack/react-query";
import { api, type EventName, type EventPayloads, type IssuesChanged } from "../api";
import { SETTINGS_KEYS } from "./settings";
import { NAV_KEY, SIDEBAR_KEY, VIEWS_KEY } from "./sidebar";
import { SYNC_KEY } from "./sync";
import { ISSUE_KEY, issueKey } from "./issues";
import { PINS_KEY } from "./pins";
import { COLUMNS_KEY, FILTER_OPTIONS_KEY, LIST_CONDITIONS_KEY, SPACE_CONFIG_KEY, STATUS_STYLES_KEY, VIEW_ROWS_KEY } from "./views";

const issueKeys = ({ ids }: IssuesChanged): QueryKey[] => ids.map(issueKey);

/** Everything an issue in the cache can show up in; a cache reset (`all`) also empties the project list. */
const ANY_ISSUE: QueryKey[] = [SETTINGS_KEYS.projects, SETTINGS_KEYS.scope, SYNC_KEY, SIDEBAR_KEY, VIEW_ROWS_KEY, COLUMNS_KEY, STATUS_STYLES_KEY, LIST_CONDITIONS_KEY, FILTER_OPTIONS_KEY, ISSUE_KEY, PINS_KEY];

type Invalidation<E extends EventName> = QueryKey[] | ((payload: EventPayloads[E]) => QueryKey[]);

/** The query keys each Rust event makes stale, or how its payload names them. Features add their events and
 *  keys here. */
export const INVALIDATIONS: { [E in EventName]?: Invalidation<E> } = {
  settings_changed: [SETTINGS_KEYS.settings, SETTINGS_KEYS.connection, SETTINGS_KEYS.scope],
  // Tracking a project changes the Spaces. A deleted list or view moves the place, so Rust's place is asked
  // again too. Rows depend on the lists, the views, the Spaces and the status styles.
  local_changed: [SETTINGS_KEYS.projects, SETTINGS_KEYS.scope, SIDEBAR_KEY, VIEWS_KEY, NAV_KEY, VIEW_ROWS_KEY, COLUMNS_KEY, SPACE_CONFIG_KEY, STATUS_STYLES_KEY, LIST_CONDITIONS_KEY, FILTER_OPTIONS_KEY, ISSUE_KEY, PINS_KEY],
  // Named issues change their rows, the counts and their own detail; filter options, columns and styles stay.
  issues_changed: (event) =>
    event.all ? ANY_ISSUE : [VIEW_ROWS_KEY, SIDEBAR_KEY, PINS_KEY, SETTINGS_KEYS.projects, SYNC_KEY, ...issueKeys(event)],
  issue_details_changed: issueKeys,
  sync_state: [SYNC_KEY],
  keybindings_changed: [SETTINGS_KEYS.keybindings],
};

/** Subscribes to every event of `INVALIDATIONS` while mounted and refetches the queries it names; `nav_changed`
 *  carries the new place itself. */
export function useEvents() {
  const client = useQueryClient();
  useEffect(() => {
    const unlisten = Object.entries(INVALIDATIONS).map(([event, invalidation]) =>
      api.listen(event as EventName, (payload) => {
        const keys = typeof invalidation === "function" ? (invalidation as (p: unknown) => QueryKey[])(payload) : invalidation;
        // A refetch already running finishes and counts; a burst of events doesn't restart it.
        for (const queryKey of keys) void client.invalidateQueries({ queryKey }, { cancelRefetch: false });
      }),
    );
    unlisten.push(api.listen("nav_changed", (nav) => client.setQueryData(NAV_KEY, nav)));
    return () => unlisten.forEach((p) => void p.then((fn) => fn()).catch(() => {}));
  }, [client]);
}
