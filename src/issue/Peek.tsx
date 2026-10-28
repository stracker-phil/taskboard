import { ExternalLink, Maximize2, Pin, X } from "lucide-react";
import { useEffect, useRef } from "react";
import type { IssueDetail } from "../api";
import { Avatar } from "../components/issue/Avatar";
import { EpicChip } from "../components/issue/EpicChip";
import { PriorityFlag } from "../components/issue/PriorityFlag";
import { StatusPill } from "../components/issue/StatusPill";
import { useIssue } from "../queries/issues";
import { useIssueActions } from "../views/list/useIssueActions";

/** Jira's description as text, a line per paragraph; the Issue view shows it as sanitised HTML. */
function plainText(html: string): string {
  const withBreaks = html.replace(/<\/(p|div|li|h[1-6])>|<br\s*\/?>/gi, (tag) => `${tag}\n`);
  return new DOMParser().parseFromString(withBreaks, "text/html").body.textContent?.trim() ?? "";
}

/** The 540px panel beside the list (artboard 4): the selected issue at a glance. */
export function Peek({ id, onClose }: { id: string; onClose: () => void }) {
  const { data: issue, isError } = useIssue(id);
  const actions = useIssueActions();
  const bodyRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (bodyRef.current) bodyRef.current.scrollTop = 0;
  }, [id]);

  return (
    <aside className="peek" aria-label="Issue peek">
      <header className="peek__header">
        {issue && (
          <>
            <span className="peek__key">{issue.key}</span>
            <span className="peek__dot" aria-hidden="true">
              ·
            </span>
            <span>{issue.issueType}</span>
          </>
        )}
        <span className="peek__spacer" />
        <button type="button" className="peek__btn" aria-label="Pin" title="Pin (P)" disabled>
          <Pin size={14} strokeWidth={1.9} aria-hidden="true" />
        </button>
        {issue && actions.openInJira && (
          <button type="button" className="peek__btn" aria-label="Open in Jira" title="Open in Jira (⌘↩)" onClick={() => actions.openInJira?.(issue.key)}>
            <ExternalLink size={14} strokeWidth={1.9} aria-hidden="true" />
          </button>
        )}
        <button type="button" className="peek__btn" aria-label="Full view" title="Full view (↩)" disabled>
          <Maximize2 size={14} strokeWidth={1.9} aria-hidden="true" />
        </button>
        <button type="button" className="peek__btn" aria-label="Close (esc)" onClick={onClose}>
          <X size={18} strokeWidth={1.9} aria-hidden="true" />
        </button>
      </header>
      <div ref={bodyRef} className="peek__body">
        {issue ? <Body issue={issue} /> : isError ? <p className="peek__missing">This issue is no longer in the cache.</p> : null}
      </div>
      <footer className="peek__footer">
        <span>J / K next</span>
        <span>↩ full view</span>
        <span>⌘↩ Jira</span>
        <span className="peek__spacer" />
        <span>esc close</span>
      </footer>
    </aside>
  );
}

function Body({ issue }: { issue: IssueDetail }) {
  const description = issue.descriptionHtml ? plainText(issue.descriptionHtml) : "";
  return (
    <>
      <h2 className="peek__title">{issue.summary}</h2>
      <div className="peek__chips">
        <StatusPill style={issue.style} name={issue.statusName} variant="peek" />
        {issue.epic && <EpicChip epic={issue.epic} />}
        {issue.releases.map((release) => (
          <span key={release} className="release-chip">
            {release}
          </span>
        ))}
        {issue.priority && (
          <span className="peek__meta">
            <PriorityFlag level={issue.priority.level} size={13} />
            {issue.priority.name}
          </span>
        )}
        {issue.assignees.map((person) => (
          <span key={person.accountId} className="peek__meta">
            <Avatar person={person} size={22} />
            {person.name}
          </span>
        ))}
      </div>
      {description ? <p className="peek__description">{description}</p> : <p className="peek__empty">No description.</p>}
    </>
  );
}
