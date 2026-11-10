import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback } from "react";
import { api, type Inbox } from "../api";
import { attempt } from "../state/requests";
import { useInboxState } from "./inboxState";

export const INBOX_KEY = ["inbox"] as const;
export const INBOX_COUNT_KEY = ["inboxCount"] as const;

const EMPTY: Inbox = { rows: [], unread: 0 };

/** The bell's badge: how many issues are unread. */
export function useInboxCount(): number {
  return useQuery({ queryKey: INBOX_COUNT_KEY, queryFn: api.inboxCount }).data ?? 0;
}

/** The open inbox's rows; Rust keeps the snapshot taken when it opened. */
export function useInboxRows(): Inbox {
  const open = useInboxState((s) => s.open);
  return useQuery({ queryKey: INBOX_KEY, queryFn: api.inboxRows, enabled: open }).data ?? EMPTY;
}

/** Opens the inbox on a new snapshot, or closes it and forgets the snapshot. */
export function useInboxToggle() {
  const client = useQueryClient();
  const show = useCallback(async () => {
    const inbox = await attempt(api.inboxOpen(), "Could not open the notifications");
    if (inbox === null) return;
    client.setQueryData(INBOX_KEY, inbox);
    useInboxState.getState().setOpen(true);
  }, [client]);
  const close = useCallback(() => {
    if (!useInboxState.getState().open) return;
    useInboxState.getState().setOpen(false);
    client.removeQueries({ queryKey: INBOX_KEY });
    void attempt(api.inboxClose(), "Could not close the notifications");
  }, [client]);
  const toggle = useCallback(() => (useInboxState.getState().open ? close() : void show()), [show, close]);
  return { show, close, toggle };
}
