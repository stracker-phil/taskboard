import type { InboxRow as Row } from "../api";
import { StatusSquare } from "../components/issue/StatusSquare";
import { timeAgo } from "../lib/relativeTime";

interface Props {
  row: Row;
  selected: boolean;
  onHover: () => void;
  onOpen: (inNewTab: boolean) => void;
}

/** One unread issue: status, key and title, then what changed and when; dimmed once read. */
export function InboxRow({ row, selected, onHover, onOpen }: Props) {
  const className = ["inbox__row", selected && "inbox__row--selected", !row.unread && "inbox__row--read"].filter(Boolean).join(" ");
  return (
    <li className={className} aria-current={selected ? "true" : undefined} onMouseEnter={onHover} onClick={(e) => onOpen(e.metaKey)}>
      <span className="inbox__line">
        <StatusSquare style={row.style} />
        <span className="inbox__key">{row.key}</span>
        <span className="inbox__summary">{row.summary}</span>
      </span>
      <span className="inbox__note">
        <span className="inbox__note-text">{row.note}</span>
        <span className="inbox__time">{timeAgo(row.changedAt)}</span>
      </span>
    </li>
  );
}
