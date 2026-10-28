import type { ListNode } from "../../api";
import { HOME_ICONS } from "../levels";
import { ListCount } from "./ListCount";

interface Props {
  rows: ListNode[];
  activeId: number | null;
  open: (listId: number) => void;
}

/** Everything, Recently changed, Pinned and the global filter lists: icon, label, count. */
export function HomeRows({ rows, activeId, open }: Props) {
  return (
    <nav className="sb-home" aria-label="Home">
      {rows.map((node) => {
        const Icon = HOME_ICONS[node.kind];
        const active = node.id === activeId;
        return (
          <button
            key={node.id}
            type="button"
            className="sb-row sb-home__row"
            data-state={active ? "active" : undefined}
            aria-current={active ? "page" : undefined}
            onClick={() => open(node.id)}
          >
            <Icon size={17} strokeWidth={1.8} aria-hidden="true" />
            <span className="sb-row__label">{node.name}</span>
            <ListCount list={node} />
          </button>
        );
      })}
    </nav>
  );
}
