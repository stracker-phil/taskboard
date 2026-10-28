import { ChevronDown } from "lucide-react";
import type { Group, GroupMark } from "../../api";
import { Avatar } from "../../components/issue/Avatar";
import { PriorityFlag } from "../../components/issue/PriorityFlag";

interface Props {
  group: Group;
  open: boolean;
  /** False for an ungrouped view: column headers only. */
  tab: boolean;
  /** The pointer or focus is in this group, which shows its chevron. */
  active: boolean;
  headers: string[];
  onToggle: () => void;
}

/** The row above a group's card: the tab with the count, then the column headers on the shared grid. */
export function GroupHeader({ group, open, tab, active, headers, onToggle }: Props) {
  const className = ["list-group", active && "list-group--active"].filter(Boolean).join(" ");
  return (
    <div className={className} role="row">
      {tab && (
        <button
          type="button"
          className="list-group__chevron"
          aria-label={`${open ? "Collapse" : "Expand"} ${group.label}`}
          aria-expanded={open}
          onClick={onToggle}
        >
          <ChevronDown size={12} strokeWidth={2.6} aria-hidden="true" />
        </button>
      )}
      {/* Only the List's header row names columns; the Table has its own header. */}
      <span className="list-group__title" role={headers.length > 0 ? "columnheader" : undefined}>
        {tab && (
          <>
            <button type="button" className={tabClass(group.mark, open)} onClick={onToggle} tabIndex={-1}>
              <Mark mark={group.mark} />
              {group.label}
            </button>
            <span className="list-group__count">{group.count}</span>
          </>
        )}
      </span>
      {headers.map((name, index) => (
        <span key={index} className="list-group__header" role="columnheader">
          {name}
        </span>
      ))}
    </div>
  );
}

function tabClass(mark: GroupMark | null, open: boolean): string {
  const style = mark?.kind === "status" ? mark.style : "todo";
  return ["status-pill", "status-pill--tab", `status-pill--${style}`, !open && "status-pill--closed"].filter(Boolean).join(" ");
}

/** What a non-status grouping shows inside its tab. */
function Mark({ mark }: { mark: GroupMark | null }) {
  switch (mark?.kind) {
    case "space":
      return (
        <span className="space-tile space-tile--20" style={{ background: `var(--space-tile-${mark.color ?? "1"})` }} aria-hidden="true">
          {mark.letter}
        </span>
      );
    case "person":
      return <Avatar person={mark.person} size={20} />;
    case "epic":
      return <span className={`list-group__epic epic-chip--${mark.color}`} aria-hidden="true" />;
    case "priority":
      return <PriorityFlag level={mark.level} />;
    default:
      return null;
  }
}
