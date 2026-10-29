import { CircleDot, Bookmark } from "lucide-react";
import { Fragment, useState } from "react";
import { api, type ActivityItem, type ChangeValue } from "../api";
import { Avatar } from "../components/issue/Avatar";
import { StatusPill } from "../components/issue/StatusPill";
import { formatLocalTime } from "../lib/time";
import { useIssueActivity } from "../queries/issues";
import { usePinOf } from "../queries/pins";
import { PinButton } from "../pins/PinButton";
import { attempt } from "../state/requests";
import { JiraHtml } from "./JiraHtml";
import { holdUnread } from "./useSeenAfter";
import { ACTIVITY_WIDTH, useActivityWidth } from "./useActivityWidth";

type Filter = "all" | "comments" | "changes";
const FILTERS: { id: Filter; label: string }[] = [
  { id: "all", label: "All" },
  { id: "comments", label: "Comments" },
  { id: "changes", label: "Changes" },
];

/** Jira's other field names show lowercased ("Fix Version" → "fix version"). */
const FIELD_NAMES: Record<string, string> = { due_date: "due date", duedate: "due date" };
const fieldName = (field: string) => FIELD_NAMES[field] ?? field.toLowerCase();

function Value({ value }: { value: ChangeValue | null }) {
  if (!value) return <strong>none</strong>;
  switch (value.kind) {
    case "status":
      return <StatusPill style={value.value.style} name={value.value.name} variant="activity" />;
    case "person":
      return <strong>{value.value.name}</strong>;
    case "text":
      return <strong>{value.value}</strong>;
  }
}

type Comment = Extract<ActivityItem, { kind: "comment" }>;

/** Shown on hover: this item and everything newer count as new again, and stay so while the issue is open. */
function MarkUnread({ issueId, at, className }: { issueId: string; at: string; className: string }) {
  const mark = () => {
    holdUnread(issueId);
    void attempt(api.issueMarkUnread(issueId, at), "Could not mark it unread");
  };
  return (
    <button type="button" className={className} aria-label="Mark unread from here" title="Mark unread from here" onClick={mark}>
      <CircleDot size={14} strokeWidth={1.9} aria-hidden="true" />
    </button>
  );
}

/** A comment with its hover actions; a bookmarked one shows the amber strip with the reason and the time left. */
function CommentCard({ comment, issueId }: { comment: Comment; issueId: string }) {
  const pin = usePinOf("comment", comment.id);
  const author = comment.author?.name ?? "Someone";
  const time = formatLocalTime(comment.at);
  return (
    <article className={comment.fresh ? "comment-card comment-card--new" : "comment-card"}>
      <header className="comment-card__head">
        {comment.author && <Avatar person={comment.author} size={26} />}
        <strong>{author}</strong>
        <time className="comment-card__time" dateTime={comment.at}>
          {time}
          {comment.edited && " · edited"}
        </time>
        <span className="comment-card__spacer" />
        <MarkUnread issueId={issueId} at={comment.at} className="comment-card__pin" />
        <PinButton kind="comment" target={comment.id} label="Bookmark comment" meta={`${author} · ${time}`} className="comment-card__pin" />
      </header>
      {pin && (
        <div className="pin-strip">
          <Bookmark size={12} strokeWidth={2} aria-hidden="true" />
          <span className="pin-strip__reason">{pin.reason ?? "Bookmarked"}</span>
          {pin.timeLeft && <span>{pin.timeLeft}</span>}
        </div>
      )}
      {comment.bodyHtml ? <JiraHtml html={comment.bodyHtml} issueId={issueId} className="comment-card__body" /> : null}
    </article>
  );
}

type Change = Extract<ActivityItem, { kind: "change" }>;

/** "Created the issue", "Changed status To do → In progress", or without values (long text such as the
 *  description) "Changed description". After the author's name the verb starts lowercase. */
function ChangeText({ item, lowercase }: { item: Change; lowercase: boolean }) {
  const verb = (text: string) => (lowercase ? text.toLowerCase() : text);
  if (item.field === "created") return <>{verb("Created")} the issue</>;
  const name = fieldName(item.field);
  if (item.from === null && item.to === null) return <>{`${verb("Changed")} ${name}`}</>;
  return (
    <>
      {verb("Changed")} {name} <Value value={item.from} /> → <Value value={item.to} />
    </>
  );
}

export function ActivityEntry({ item, issueId }: { item: ActivityItem; issueId: string }) {
  if (item.kind === "comment") return <CommentCard comment={item} issueId={issueId} />;
  return (
    <div className={item.fresh ? "change-line change-line--new" : "change-line"}>
      <span className="change-line__text">
        {item.author && <strong>{item.author.name} </strong>}
        <ChangeText item={item} lowercase={item.author !== null} />
      </span>
      <time className="change-line__time" dateTime={item.at}>
        {formatLocalTime(item.at)}
      </time>
      <MarkUnread issueId={issueId} at={item.at} className="comment-card__pin change-line__action" />
    </div>
  );
}

/** Comments and recorded changes, newest first; the new ones sit under a divider at the top. */
function ActivityList({ items, issueId }: { items: ActivityItem[]; issueId: string }) {
  const firstNew = items.findIndex((item) => item.fresh);
  return items.map((item, i) => (
    <Fragment key={item.kind === "comment" ? item.id : `${item.field}:${item.at}:${i}`}>
      {i === firstNew && (
        <div className="activity__divider" role="separator">
          New since yesterday
        </div>
      )}
      <ActivityEntry item={item} issueId={issueId} />
    </Fragment>
  ));
}

export function Activity({ id }: { id: string }) {
  const [filter, setFilter] = useState<Filter>("all");
  const { data: items = [] } = useIssueActivity(id);
  const shown = items.filter((item) => filter === "all" || (filter === "comments") === (item.kind === "comment"));
  const { width, dragging, handle } = useActivityWidth();
  return (
    <aside className="activity" aria-label="Activity" style={{ width }} data-resizing={dragging || undefined}>
      <div
        className="activity__resize"
        role="separator"
        aria-orientation="vertical"
        aria-label="Activity width"
        aria-valuenow={width}
        aria-valuemin={ACTIVITY_WIDTH.min}
        aria-valuemax={ACTIVITY_WIDTH.max}
        tabIndex={0}
        {...handle}
      />
      <header className="activity__header">
        <h2>Activity</h2>
        <div className="segmented" role="group" aria-label="Show">
          {FILTERS.map((f) => (
            <button key={f.id} type="button" className="segmented__btn" aria-pressed={filter === f.id} onClick={() => setFilter(f.id)}>
              {f.label}
            </button>
          ))}
        </div>
      </header>
      <div className="activity__list">
        {shown.length === 0 ? <p className="activity__empty">Nothing here yet.</p> : <ActivityList items={shown} issueId={id} />}
      </div>
    </aside>
  );
}
