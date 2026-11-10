import type { CSSProperties } from "react";
import { Avatar } from "../../components/issue/Avatar";
import { EpicChip } from "../../components/issue/EpicChip";
import { PriorityFlag } from "../../components/issue/PriorityFlag";
import { StatusPill } from "../../components/issue/StatusPill";
import { hasContent } from "../../lib/jiraHtml";
import { useIssue } from "../../queries/issues";
import { JiraHtml } from "../JiraHtml";

export const WIDTH = 380;
export const MAX_HEIGHT = 360;
const GAP = 10;
const MARGIN = 8;

/** Beside `rect`: on the preferred side when there is room, else on the other; with room on neither, over the
 *  left part of the window, never off-screen. */
export function place(rect: DOMRect, prefer: "right" | "left" = "right"): CSSProperties {
  const fitsRight = rect.right + GAP + WIDTH + MARGIN <= window.innerWidth;
  const fitsLeft = rect.left - GAP - WIDTH >= MARGIN;
  const right = prefer === "right" ? fitsRight : !fitsLeft && fitsRight;
  const left = right ? rect.right + GAP : Math.max(MARGIN, rect.left - GAP - WIDTH);
  const top = Math.max(MARGIN, Math.min(rect.top, window.innerHeight - MAX_HEIGHT - MARGIN));
  return { left, top, width: WIDTH, maxHeight: MAX_HEIGHT };
}

interface Props {
  id: string;
  style: CSSProperties;
  /** Scrolls and takes the pointer (the inbox's preview); the ⌥-hover card never does. */
  interactive?: boolean;
}

/** A small card with an issue's summary, status, people and description: the ⌥-hover peek and the inbox's
 *  preview. */
export function PeekCard({ id, style, interactive = false }: Props) {
  const { data: issue } = useIssue(id);
  if (!issue) return null;
  return (
    <aside
      className={interactive ? "hover-peek hover-peek--interactive" : "hover-peek"}
      role={interactive ? "region" : "tooltip"}
      aria-label={`Peek at ${issue.key}`}
      style={style}
    >
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
        <JiraHtml html={issue.descriptionHtml} issueId={issue.id} className="hover-peek__description" />
      ) : (
        <p className="hover-peek__empty">No description.</p>
      )}
    </aside>
  );
}
