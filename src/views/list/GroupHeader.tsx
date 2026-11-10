import { DisclosureChevron } from "../../components/DisclosureChevron";
import type { Group, GroupMark } from "../../api";
import { Avatar } from "../../components/issue/Avatar";
import { PriorityFlag } from "../../components/issue/PriorityFlag";
import { Tile } from "../../components/look/Tile";
import type { GroupPicks } from "./picking";
import { GroupBox } from "./PickBox";

interface Props {
  group: Group;
  open: boolean;
  /** False for an ungrouped view: column headers only. */
  tab: boolean;
  /** The pointer or focus is in this group, which shows its chevron. */
  active: boolean;
  /** The List's header rows name the columns; the Table has its own header. */
  namesColumns: boolean;
  headers: string[];
  onToggle: () => void;
  /** How many of the group's rows are picked; its checkbox picks or unpicks them all. */
  picks: GroupPicks;
  onTogglePicks: () => void;
}

/** The row above a group's card: its checkbox in the rows' checkbox column, the tab with the count, then the column
 *  headers on the shared grid. */
export function GroupHeader({ group, open, tab, active, namesColumns, headers, onToggle, picks, onTogglePicks }: Props) {
  const className = ["list-group", active && "list-group--active"].filter(Boolean).join(" ");
  return (
    <div className={className} role="row">
      <GroupBox group={tab ? group.label : null} picks={picks} onToggle={onTogglePicks} className="list-group__pick" />
      {tab && (
        <button
          type="button"
          className="list-group__chevron"
          aria-label={`${open ? "Collapse" : "Expand"} ${group.label}`}
          aria-expanded={open}
          onClick={onToggle}
        >
          <DisclosureChevron open={open} />
        </button>
      )}
      <span className="list-group__title" role={namesColumns ? "columnheader" : undefined}>
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

/** What a non-status grouping shows inside its tab (the List's group tab, a Board column's title). */
export function Mark({ mark }: { mark: GroupMark | null }) {
  switch (mark?.kind) {
    case "space":
      return <Tile icon={mark.icon} color={mark.color ?? "1"} fallback={mark.initial} size={20} />;
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
