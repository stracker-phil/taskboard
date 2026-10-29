import type { CSSProperties } from "react";
import { createPortal } from "react-dom";
import { Avatar } from "../../components/issue/Avatar";
import { EpicChip } from "../../components/issue/EpicChip";
import { PriorityFlag } from "../../components/issue/PriorityFlag";
import { StatusPill } from "../../components/issue/StatusPill";
import { hasContent } from "../../lib/jiraHtml";
import { useIssue } from "../../queries/issues";
import { JiraHtml } from "../JiraHtml";
import { type PeekTarget, useHoverPeek } from "./useHoverPeek";

const WIDTH = 380;
const MAX_HEIGHT = 360;
const GAP = 10;
const MARGIN = 8;

/** Beside the issue's element: on its right when there is room, else on its left; kept inside the window. */
function place(rect: DOMRect): CSSProperties {
  const right = rect.right + GAP + WIDTH + MARGIN <= window.innerWidth;
  const left = right ? rect.right + GAP : Math.max(MARGIN, rect.left - GAP - WIDTH);
  const top = Math.max(MARGIN, Math.min(rect.top, window.innerHeight - MAX_HEIGHT - MARGIN));
  return { left, top, width: WIDTH, maxHeight: MAX_HEIGHT };
}

/** ⌥ + hover over any issue: a small card with its summary, status, people and description, without leaving
 *  the place. It never takes the pointer, so the hovered element keeps it. */
export function HoverPeek() {
  const target = useHoverPeek();
  return target ? createPortal(<PeekCard target={target} />, document.body) : null;
}

function PeekCard({ target }: { target: PeekTarget }) {
  const { data: issue } = useIssue(target.id);
  if (!issue) return null;
  return (
    <aside className="hover-peek" role="tooltip" aria-label={`Peek at ${issue.key}`} style={place(target.rect)}>
      <div className="hover-peek__meta">
        <span className="hover-peek__key">{issue.key}</span>
        <span aria-hidden="true">·</span>
        <span>{issue.issueType}</span>
      </div>
      <h2 className={issue.gone ? "hover-peek__title issue-title--gone" : "hover-peek__title"}>{issue.summary}</h2>
      <div className="hover-peek__chips">
        <StatusPill style={issue.style} name={issue.statusName} variant="peek" />
        {issue.epic && <EpicChip epic={issue.epic} />}
        {issue.priority && (
          <span className="hover-peek__chip">
            <PriorityFlag level={issue.priority.level} size={12} />
            {issue.priority.name}
          </span>
        )}
        {issue.assignees.map((person) => (
          <span key={person.accountId} className="hover-peek__chip">
            <Avatar person={person} size={18} />
            {person.name}
          </span>
        ))}
      </div>
      {issue.descriptionHtml && hasContent(issue.descriptionHtml) ? (
        <JiraHtml html={issue.descriptionHtml} className="hover-peek__description" />
      ) : (
        <p className="hover-peek__empty">No description.</p>
      )}
    </aside>
  );
}
