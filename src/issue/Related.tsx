import { ChevronRight } from "lucide-react";
import { useState } from "react";
import type { IssueLink, Related as RelatedData, RelatedIssue } from "../api";
import { Avatar } from "../components/issue/Avatar";
import { StatusSquare } from "../components/issue/StatusSquare";

/** "Blocks NW-1051 …": the issue's links; a cached linked issue opens in this view. */
export function Links({ links, onOpen }: { links: IssueLink[]; onOpen: (id: string) => void }) {
  if (links.length === 0) return null;
  return (
    <section className="issue-links" aria-label="Links">
      {links.map((link) => (
        <div key={`${link.relation}:${link.issue.key}`} className="issue-links__row">
          <span className="issue-links__relation">{link.relation}</span>
          <IssueButton issue={link.issue} onOpen={onOpen} compact />
        </div>
      ))}
    </section>
  );
}

function IssueButton({ issue, onOpen, compact = false }: { issue: RelatedIssue; onOpen: (id: string) => void; compact?: boolean }) {
  const id = issue.id;
  return (
    <button
      type="button"
      className={["related-issue", compact && "related-issue--compact", issue.changed && "related-issue--changed"].filter(Boolean).join(" ")}
      disabled={id === null}
      data-issue-id={id ?? undefined}
      onClick={() => id && onOpen(id)}
    >
      <StatusSquare style={issue.style} />
      <span className="related-issue__key">{issue.key}</span>
      <span className="related-issue__summary">{issue.summary}</span>
      {!compact && <span className="related-issue__type">{issue.issueType}</span>}
      {!compact && issue.assignee && <Avatar person={issue.assignee} size={24} />}
    </button>
  );
}

/** Child issues of an epic (or subtasks of a task): open ones first, the done ones behind "Show N done". Hidden
 *  when there are none. */
export function Children({
  related,
  isEpic,
  onOpen,
  onOpenAsList,
}: {
  related: RelatedData;
  isEpic: boolean;
  onOpen: (id: string) => void;
  onOpenAsList: () => void;
}) {
  const [showDone, setShowDone] = useState(false);
  const { open, done } = related.children;
  const count = open.length + done.length;
  if (count === 0) return null;
  const title = isEpic ? "Child issues" : "Subtasks";
  return (
    <section className="children" aria-label={title}>
      <header className="children__header">
        <h2>{title}</h2>
        <span className="children__count">{count}</span>
        <span className="children__spacer" />
        {isEpic && (
          <button type="button" className="btn-secondary-small" onClick={onOpenAsList}>
            Open as list view
          </button>
        )}
      </header>
      {open.map((issue) => (
        <IssueButton key={issue.key} issue={issue} onOpen={onOpen} />
      ))}
      {done.length > 0 && (
        <>
          <button type="button" className="children__toggle" aria-expanded={showDone} onClick={() => setShowDone((s) => !s)}>
            <ChevronRight size={12} strokeWidth={2.4} className="children__chevron" aria-hidden="true" />
            {showDone ? `Hide ${done.length} done` : `Show ${done.length} done`}
          </button>
          {showDone && (
            <div className="children__done">
              {done.map((issue) => (
                <IssueButton key={issue.key} issue={issue} onOpen={onOpen} />
              ))}
            </div>
          )}
        </>
      )}
    </section>
  );
}
