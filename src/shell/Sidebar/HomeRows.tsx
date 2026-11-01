import type { ListNode } from "../../api";
import { Tile } from "../../components/look/Tile";
import { HOME_ICONS } from "../levels";
import { ListCount } from "./ListCount";

interface Props {
  rows: ListNode[];
  activeId: number | null;
  open: (listId: number) => void;
}

/** Everything, Bookmarks and the global filter lists: icon, label, count. */
export function HomeRows({ rows, activeId, open }: Props) {
  return (
    <nav className="sb-home" aria-label="Home">
      {rows.map((node) => {
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
            {node.kind === "list" ? (
              <Tile icon={node.icon} color={node.color} fallback={HOME_ICONS.list} size={node.color ? 18 : 17} />
            ) : (
              <HomeIcon kind={node.kind} />
            )}
            <span className="sb-row__label">{node.name}</span>
            <ListCount list={node} />
          </button>
        );
      })}
    </nav>
  );
}

function HomeIcon({ kind }: { kind: ListNode["kind"] }) {
  const Icon = HOME_ICONS[kind];
  return <Icon size={17} strokeWidth={1.8} aria-hidden="true" />;
}
