import { ExternalLink, Maximize2, X } from "lucide-react";
import { CopyKey } from "../components/issue/CopyKey";
import { useEffect, useRef } from "react";
import { useShortcuts } from "../commands/useShortcuts";
import type { IssueDetail } from "../api";
import { Avatar } from "../components/issue/Avatar";
import { EpicChip } from "../components/issue/EpicChip";
import { PriorityFlag } from "../components/issue/PriorityFlag";
import { PriorityMenuButton } from "../writes/PriorityMenuButton";
import { EditableTitle } from "../writes/EditableTitle";
import { useIssue, useIssueActivity } from "../queries/issues";
import { ActivityEntry } from "./Activity";
import { hasContent } from "../lib/jiraHtml";
import { JiraHtml } from "./JiraHtml";
import { RichFields } from "./RichFields";
import { GoneNotice } from "./GoneNotice";
import { IssueMissing } from "./IssueMissing";
import { SinceBanner } from "./SinceBanner";
import { usePreviewedAfter, useSeenAfter } from "./useSeenAfter";
import { WebLinks } from "./WebLinks";
import { PinButton } from "../pins/PinButton";
import { useIssueActions } from "../views/list/useIssueActions";
import { StatusMenuButton } from "../writes/StatusMenuButton";

/** A footer hint ("J / K next"); none while its keys are unbound. */
function KeyHint({ keys, text }: { keys: string[]; text: string }) {
  const bound = keys.filter(Boolean);
  return bound.length > 0 ? <span>{`${bound.join(" / ")} ${text}`}</span> : null;
}

/** The 540px panel beside the list (artboard 4): the selected issue at a glance. */
export function Peek({ id, onClose, onExpand }: { id: string; onClose: () => void; onExpand: () => void }) {
  const { data: issue, error } = useIssue(id);
  const actions = useIssueActions();
  const bodyRef = useRef<HTMLDivElement>(null);
  const { key, title } = useShortcuts();
  useSeenAfter(id, issue?.unread ?? false);
  usePreviewedAfter(id);

  useEffect(() => {
    if (bodyRef.current) bodyRef.current.scrollTop = 0;
  }, [id]);

  return (
    <aside className="peek" aria-label="Issue preview">
      <header className="peek__header">
        {issue && (
          <>
            <CopyKey className="peek__key" issueKey={issue.key} />
            <span className="peek__dot" aria-hidden="true">
              ·
            </span>
            <span>{issue.issueType}</span>
          </>
        )}
        <span className="peek__spacer" />
        {issue && <PinButton kind="issue" target={issue.id} label="Bookmark this issue" meta={issue.key} command="issue.togglePin" className="peek__btn" />}
        {issue && actions.openInJira && (
          <button type="button" className="peek__btn" aria-label="Open in Jira" title={title("Open in Jira", "issue.openInJira")} onClick={() => actions.openInJira?.(issue.key)}>
            <ExternalLink size={14} strokeWidth={1.9} aria-hidden="true" />
          </button>
        )}
        <button type="button" className="peek__btn" aria-label="Full view" title={title("Full view", "issue.open")} onClick={onExpand}>
          <Maximize2 size={14} strokeWidth={1.9} aria-hidden="true" />
        </button>
        <button type="button" className="peek__btn" aria-label="Close" title={title("Close", "peek.close")} onClick={onClose}>
          <X size={18} strokeWidth={1.9} aria-hidden="true" />
        </button>
      </header>
      <div ref={bodyRef} className="peek__body">
        {issue ? <Body issue={issue} onExpand={onExpand} onClose={onClose} /> : error ? <IssueMissing error={error} className="peek__missing" /> : null}
      </div>
      <footer className="peek__footer">
        <KeyHint keys={[key("issue.next"), key("issue.previous")]} text="next" />
        <KeyHint keys={[key("issue.open")]} text="full view" />
        <KeyHint keys={[key("issue.openInJira")]} text="Jira" />
        <span className="peek__spacer" />
        <KeyHint keys={[key("peek.close")]} text="close" />
      </footer>
    </aside>
  );
}

const LATEST = 3;

function Body({ issue, onExpand, onClose }: { issue: IssueDetail; onExpand: () => void; onClose: () => void }) {
  const { data: activity = [] } = useIssueActivity(issue.id);
  const latest = activity.slice(0, LATEST);
  return (
    <>
      <EditableTitle key={issue.id} issue={issue} as="h2" className="peek__title" />
      {issue.gone && <GoneNotice issue={issue} onRemoved={onClose} />}
      <div className="peek__chips">
        <StatusMenuButton issue={issue} variant="peek" />
        {issue.epic && <EpicChip epic={issue.epic} />}
        {issue.releases.map((release) => (
          <span key={release} className="release-chip">
            {release}
          </span>
        ))}
        {issue.priority && (
          <PriorityMenuButton issue={issue}>
            <span className="peek__meta">
              <PriorityFlag level={issue.priority.level} size={13} />
              {issue.priority.name}
            </span>
          </PriorityMenuButton>
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
        <JiraHtml html={issue.descriptionHtml} issueId={issue.id} className="peek__description" />
      ) : (
        <p className="peek__empty">No description.</p>
      )}
      <RichFields fields={issue.custom} issueId={issue.id} variant="peek" />
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
            <ActivityEntry key={i} item={item} issueId={issue.id} />
          ))}
        </section>
      )}
    </>
  );
}
