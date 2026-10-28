import { CheckCheck, Copy, ExternalLink, Pin } from "lucide-react";
import type { MouseEvent } from "react";
import { usePinOf } from "../../queries/pins";

export interface RowActionHandlers {
  copyKey: (key: string) => void;
  /** Null without a Jira site. */
  openInJira: ((key: string) => void) | null;
  markSeen: (id: string) => void;
  /** Pins until unpinned, or unpins. */
  togglePin: (id: string) => void;
}

interface Props {
  issueId: string;
  issueKey: string;
  /** Has unseen changes: adds Mark seen. */
  changed: boolean;
  actions: RowActionHandlers;
}

/** The hover toolbar at the end of a row's title: Mark seen, Copy key, Pin, Open in Jira. */
export function RowActions({ issueId, issueKey, changed, actions }: Props) {
  const pinned = usePinOf("issue", issueId) !== undefined;
  // The row's own click selects it; an action must not.
  const run = (action: () => void) => (e: MouseEvent) => {
    e.stopPropagation();
    action();
  };
  return (
    <span className="row-actions">
      {changed && (
        <button type="button" className="row-actions__btn" title="Mark seen" aria-label={`Mark ${issueKey} seen`} onClick={run(() => actions.markSeen(issueId))}>
          <CheckCheck size={13} strokeWidth={2} aria-hidden="true" />
        </button>
      )}
      <button type="button" className="row-actions__btn" title="Copy key (⌘C)" aria-label={`Copy ${issueKey}`} onClick={run(() => actions.copyKey(issueKey))}>
        <Copy size={13} strokeWidth={2} aria-hidden="true" />
      </button>
      <button
        type="button"
        className="row-actions__btn"
        title={pinned ? "Unpin (P)" : "Pin (P)"}
        aria-label={pinned ? `Unpin ${issueKey}` : `Pin ${issueKey}`}
        aria-pressed={pinned}
        onClick={run(() => actions.togglePin(issueId))}
      >
        <Pin size={13} strokeWidth={2} aria-hidden="true" />
      </button>
      {actions.openInJira && (
        <button
          type="button"
          className="row-actions__btn"
          title="Open in Jira (⌘↩)"
          aria-label={`Open ${issueKey} in Jira`}
          onClick={run(() => actions.openInJira?.(issueKey))}
        >
          <ExternalLink size={13} strokeWidth={2} aria-hidden="true" />
        </button>
      )}
    </span>
  );
}
