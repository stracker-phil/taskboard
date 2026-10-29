import { ExternalLink, Maximize2, X } from "lucide-react";
import { useEffect, useRef } from "react";
import type { IssueDetail } from "../api";
import { Avatar } from "../components/issue/Avatar";
import { EpicChip } from "../components/issue/EpicChip";
import { PriorityFlag } from "../components/issue/PriorityFlag";
import { StatusPill } from "../components/issue/StatusPill";
import { useIssue, useIssueActivity } from "../queries/issues";
import { ActivityEntry } from "./Activity";
import { hasContent } from "../lib/jiraHtml";
import { JiraHtml } from "./JiraHtml";
import { RichFields } from "./RichFields";
import { GoneNotice } from "./GoneNotice";
import { SinceBanner } from "./SinceBanner";
import { useSeenAfter } from "./useSeenAfter";
import { WebLinks } from "./WebLinks";
import { PinButton } from "../pins/PinButton";
import { useIssueActions } from "../views/list/useIssueActions";

/** The 540px panel beside the list (artboard 4): the selected issue at a glance. */
export function Peek({ id, onClose, onExpand }: { id: string; onClose: () => void; onExpand: () => void }) {
  const { data: issue, isError } = useIssue(id);
  const actions = useIssueActions();
  const bodyRef = useRef<HTMLDivElement>(null);
  useSeenAfter(id, Boolean(issue?.since));

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
        {issue && <PinButton kind="issue" target={issue.id} label="Pin this issue" meta={issue.key} className="peek__btn" />}
        {issue && actions.openInJira && (
          <button type="button" className="peek__btn" aria-label="Open in Jira" title="Open in Jira (⌘↩)" onClick={() => actions.openInJira?.(issue.key)}>
            <ExternalLink size={14} strokeWidth={1.9} aria-hidden="true" />
          </button>
        )}
        <button type="button" className="peek__btn" aria-label="Full view" title="Full view (↩)" onClick={onExpand}>
          <Maximize2 size={14} strokeWidth={1.9} aria-hidden="true" />
        </button>
        <button type="button" className="peek__btn" aria-label="Close (esc)" onClick={onClose}>
          <X size={18} strokeWidth={1.9} aria-hidden="true" />
        </button>
      </header>
      <div ref={bodyRef} className="peek__body">
        {issue ? <Body issue={issue} onExpand={onExpand} onClose={onClose} /> : isError ? <p className="peek__missing">This issue is no longer in the cache.</p> : null}
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

const LATEST = 3;

function Body({ issue, onExpand, onClose }: { issue: IssueDetail; onExpand: () => void; onClose: () => void }) {
  const { data: activity = [] } = useIssueActivity(issue.id);
  const latest = activity.slice(-LATEST).reverse();
  return (
    <>
      <h2 className={issue.gone ? "peek__title issue-title--gone" : "peek__title"}>{issue.summary}</h2>
      {issue.gone && <GoneNotice issue={issue} onRemoved={onClose} />}
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
      {issue.since && <SinceBanner since={issue.since} />}
      {issue.descriptionHtml && hasContent(issue.descriptionHtml) ? (
        <JiraHtml html={issue.descriptionHtml} className="peek__description" />
      ) : (
        <p className="peek__empty">No description.</p>
      )}
      <RichFields fields={issue.custom} variant="peek" />
      <WebLinks id={issue.id} variant="peek" />
      {latest.length > 0 && (
        <section className="peek__activity" aria-label="Latest activity">
          <header className="peek__activity-header">
            <h3>Latest activity</h3>
            <button type="button" className="link-btn" onClick={onExpand}>
              All activity
            </button>
          </header>
          {latest.map((item, i) => (
            <ActivityEntry key={i} item={item} />
          ))}
        </section>
      )}
    </>
  );
}
