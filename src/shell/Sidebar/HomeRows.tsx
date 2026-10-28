import type { ListNode } from "../../api";
import { listIcon } from "../../components/listIcons";
import { HOME_ICONS } from "../levels";
import { ListCount } from "./ListCount";

interface Props {
  rows: ListNode[];
  activeId: number | null;
  open: (listId: number) => void;
}

/** Everything, Recently changed, Pinned and the global filter lists: icon, label, count. Recently changed shows
 *  its unseen issues as a badge instead. */
export function HomeRows({ rows, activeId, open }: Props) {
  return (
    <nav className="sb-home" aria-label="Home">
      {rows.map((node) => {
        const Icon = node.kind === "list" ? listIcon(node.icon) : HOME_ICONS[node.kind];
        const active = node.id === activeId;
        const unread = node.kind === "recent" && (node.count ?? 0) > 0;
        return (
          <button
            key={node.id}
            type="button"
            className="sb-row sb-home__row"
            data-state={active ? "active" : undefined}
            data-unread={unread || undefined}
            aria-current={active ? "page" : undefined}
            onClick={() => open(node.id)}
          >
            <Icon size={17} strokeWidth={1.8} aria-hidden="true" />
            <span className="sb-row__label">{node.name}</span>
            {unread ? (
              <span className="sb-row__badge" aria-label={`${node.count} unseen`}>
                {node.count}
              </span>
            ) : (
              node.kind !== "recent" && <ListCount list={node} />
            )}
          </button>
        );
      })}
    </nav>
  );
}
