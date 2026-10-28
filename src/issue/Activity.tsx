import { useState } from "react";
import type { ActivityItem, ChangeValue } from "../api";
import { Avatar } from "../components/issue/Avatar";
import { StatusPill } from "../components/issue/StatusPill";
import { formatLocalTime } from "../lib/time";
import { useIssueActivity } from "../queries/issues";
import { JiraHtml } from "./JiraHtml";

type Filter = "all" | "comments" | "changes";
const FILTERS: { id: Filter; label: string }[] = [
  { id: "all", label: "All" },
  { id: "comments", label: "Comments" },
  { id: "changes", label: "Changes" },
];

const FIELD_NAMES: Record<string, string> = { status: "status", assignee: "assignee", due_date: "due date", priority: "priority" };

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

export function ActivityEntry({ item }: { item: ActivityItem }) {
  if (item.kind === "comment") {
    return (
      <article className="comment-card">
        <header className="comment-card__head">
          {item.author && <Avatar person={item.author} size={26} />}
          <strong>{item.author?.name ?? "Someone"}</strong>
          <time className="comment-card__time" dateTime={item.at}>
            {formatLocalTime(item.at)}
            {item.edited && " · edited"}
          </time>
        </header>
        {item.bodyHtml ? <JiraHtml html={item.bodyHtml} className="comment-card__body" /> : null}
      </article>
    );
  }
  return (
    <div className="change-line">
      <span className="change-line__text">
        {item.field === "created" ? (
          "Created the issue"
        ) : (
          <>
            Changed {FIELD_NAMES[item.field] ?? item.field} <Value value={item.from} /> → <Value value={item.to} />
          </>
        )}
      </span>
      <time className="change-line__time" dateTime={item.at}>
        {formatLocalTime(item.at)}
      </time>
    </div>
  );
}

/** Comments and recorded changes, oldest first, with All / Comments / Changes. Commenting arrives with writes. */
export function Activity({ id }: { id: string }) {
  const [filter, setFilter] = useState<Filter>("all");
  const { data: items = [] } = useIssueActivity(id);
  const shown = items.filter((item) => filter === "all" || (filter === "comments") === (item.kind === "comment"));
  return (
    <aside className="activity" aria-label="Activity">
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
        {shown.length === 0 ? <p className="activity__empty">Nothing here yet.</p> : shown.map((item, i) => <ActivityEntry key={item.kind === "comment" ? item.id : `${item.field}:${item.at}:${i}`} item={item} />)}
      </div>
      <div className="activity__composer">
        <div className="activity__box" aria-disabled="true">
          Commenting comes later; reply in Jira for now.
        </div>
      </div>
    </aside>
  );
}
