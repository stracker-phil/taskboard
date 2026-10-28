import type { Group, GroupMark } from "../../api";
import { Avatar } from "../../components/issue/Avatar";
import { PriorityFlag } from "../../components/issue/PriorityFlag";
import { StatusPill } from "../../components/issue/StatusPill";

export function GroupHeader({ group, open, onToggle }: { group: Group; open: boolean; onToggle: () => void }) {
  return (
    <div className="list-group">
      <button type="button" className="list-group__chevron" aria-label={open ? "Collapse group" : "Expand group"} aria-expanded={open} onClick={onToggle}>
        {open ? "▼" : "▶"}
      </button>
      <Label group={group} mark={group.mark} />
      <span className="list-group__count">{group.count}</span>
    </div>
  );
}

function Label({ group, mark }: { group: Group; mark: GroupMark | null }) {
  switch (mark?.kind) {
    case "status":
      return <StatusPill style={mark.style} name={group.label} />;
    case "space":
      return (
        <span className="list-group__label">
          <span className="space-tile space-tile--22" style={{ background: `var(--space-tile-${mark.color ?? "1"})` }} aria-hidden="true">
            {mark.letter}
          </span>
          {group.label}
        </span>
      );
    case "person":
      return (
        <span className="list-group__label">
          <Avatar person={mark.person} size={22} />
          {group.label}
        </span>
      );
    case "epic":
      return <span className={`list-group__label epic-chip epic-chip--${mark.color}`}>{group.label}</span>;
    case "priority":
      return (
        <span className="list-group__label">
          <PriorityFlag level={mark.level} />
          {group.label}
        </span>
      );
    default:
      return <span className="list-group__label">{group.label}</span>;
  }
}
