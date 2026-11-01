import { CheckCheck, CircleDot, Copy, ExternalLink, Bookmark } from "lucide-react";
import type { MouseEvent } from "react";
import { useShortcuts } from "../../commands/useShortcuts";
import type { MenuAt } from "../../components/menu";

export interface RowActionHandlers {
  copyKey: (key: string) => void;
  /** Null without a Jira site. */
  openInJira: ((key: string) => void) | null;
  markSeen: (id: string) => void;
  markUnread: (id: string) => void;
  /** Bookmarks until removed, or removes the bookmark. */
  togglePin: (id: string) => void;
  /** Assigns the issue to me, or unassigns it when it is mine; null while changes to Jira are off. */
  toggleMine: ((id: string) => void) | null;
  /** Opens the status menu under `at`; null while changes to Jira are off. */
  changeStatus: ((issue: { id: string; key: string }, at: MenuAt) => void) | null;
  /** Opens the assignee menu under `at`; null while changes to Jira are off. */
  assignTo: ((issue: { id: string; key: string }, at: MenuAt) => void) | null;
  /** Opens the priority menu under `at`; null while changes to Jira, or to priorities, are off. */
  changePriority: ((issue: { id: string; key: string }, at: MenuAt) => void) | null;
}

interface Props {
  issueId: string;
  issueKey: string;
  /** Unread: the toggle marks it read, else unread. */
  changed: boolean;
  actions: RowActionHandlers;
  /** Has an active bookmark. */
  pinned: boolean;
}

/** The hover toolbar at the end of a row's title: Mark as read or unread, Copy key, Bookmark, Open in Jira. */
export function RowActions({ issueId, issueKey, changed, actions, pinned }: Props) {
  // The row's own click selects it; an action must not.
  const run = (action: () => void) => (e: MouseEvent) => {
    e.stopPropagation();
    action();
  };
  const { title } = useShortcuts();
  return (
    <span className="row-actions">
      {changed ? (
        <button type="button" className="row-actions__btn" title={title("Mark as read", "issue.markSeen")} aria-label={`Mark ${issueKey} as read`} onClick={run(() => actions.markSeen(issueId))}>
          <CheckCheck size={13} strokeWidth={2} aria-hidden="true" />
        </button>
      ) : (
        <button type="button" className="row-actions__btn" title={title("Mark as unread", "issue.markSeen")} aria-label={`Mark ${issueKey} as unread`} onClick={run(() => actions.markUnread(issueId))}>
          <CircleDot size={13} strokeWidth={2} aria-hidden="true" />
        </button>
      )}
      <button type="button" className="row-actions__btn" title={title("Copy key", "issue.copyKey")} aria-label={`Copy ${issueKey}`} onClick={run(() => actions.copyKey(issueKey))}>
        <Copy size={13} strokeWidth={2} aria-hidden="true" />
      </button>
      <button
        type="button"
        className="row-actions__btn"
        title={title(pinned ? "Remove bookmark" : "Bookmark", "issue.togglePin")}
        aria-label={pinned ? `Remove the bookmark of ${issueKey}` : `Bookmark ${issueKey}`}
        aria-pressed={pinned}
        onClick={run(() => actions.togglePin(issueId))}
      >
        <Bookmark size={13} strokeWidth={2} aria-hidden="true" />
      </button>
      {actions.openInJira && (
        <button
          type="button"
          className="row-actions__btn"
          title={title("Open in Jira", "issue.openInJira")}
          aria-label={`Open ${issueKey} in Jira`}
          onClick={run(() => actions.openInJira?.(issueKey))}
        >
          <ExternalLink size={13} strokeWidth={2} aria-hidden="true" />
        </button>
      )}
    </span>
  );
}
