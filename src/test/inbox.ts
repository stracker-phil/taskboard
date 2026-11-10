import type { Inbox, InboxRow } from "../api/types";

/** The ISO time of a change that happened `minutes` ago. */
export const minutesAgo = (minutes: number): string => new Date(Date.now() - minutes * 60_000).toISOString();

/** An unread inbox row of the "Website relaunch" Space; override per test. */
export function inboxRow(patch: Partial<InboxRow> = {}): InboxRow {
  return {
    id: "10042",
    key: "NW-1042",
    summary: "Redesign the pricing page",
    projectId: "10000",
    style: "review",
    statusName: "Review",
    note: "Moved to Review, 1 new comment.",
    changedAt: minutesAgo(5),
    unread: true,
    ...patch,
  };
}

/** The inbox with these rows, newest first as given; `unread` counts the unread ones. */
export const inboxOf = (rows: InboxRow[]): Inbox => ({ rows, unread: rows.filter((r) => r.unread).length });

/** Three unread issues in the order Rust lists them. */
export const THREE_ROWS: InboxRow[] = [
  inboxRow({ id: "10042", key: "NW-1042", summary: "Redesign the pricing page" }),
  inboxRow({ id: "10043", key: "NW-1043", summary: "Publish the launch notes", note: "Assigned to you.", changedAt: minutesAgo(125) }),
  inboxRow({ id: "10044", key: "NW-1044", summary: "Fix the footer links", note: "2 new comments.", changedAt: minutesAgo(30) }),
];
