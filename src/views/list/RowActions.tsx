import { Copy, ExternalLink, Pin } from "lucide-react";
import type { MouseEvent } from "react";

export interface RowActionHandlers {
  copyKey: (key: string) => void;
  /** Null without a Jira site. */
  openInJira: ((key: string) => void) | null;
}

/** The hover toolbar at the end of a row's title: Copy key, Pin (pins arrive later), Open in Jira. */
export function RowActions({ issueKey, actions }: { issueKey: string; actions: RowActionHandlers }) {
  // The row's own click selects it; an action must not.
  const run = (action: () => void) => (e: MouseEvent) => {
    e.stopPropagation();
    action();
  };
  return (
    <span className="row-actions">
      <button type="button" className="row-actions__btn" title="Copy key (⌘C)" aria-label={`Copy ${issueKey}`} onClick={run(() => actions.copyKey(issueKey))}>
        <Copy size={13} strokeWidth={2} aria-hidden="true" />
      </button>
      <button type="button" className="row-actions__btn" title="Pin (P)" aria-label={`Pin ${issueKey}`} disabled onClick={(e) => e.stopPropagation()}>
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
