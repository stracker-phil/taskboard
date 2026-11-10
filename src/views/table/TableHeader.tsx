import type { PointerEvent } from "react";
import type { Sort, SortBy } from "../../api";
import { useColumnsCatalogue } from "../../queries/views";
import type { GroupPicks } from "../list/picking";
import { GroupBox } from "../list/PickBox";
import { sortByOf } from "./sortable";

const FIXED_NAMES: Record<string, string> = { key: "Key", summary: "Summary", status: "Status" };

interface Props {
  listId: number;
  columns: string[];
  sort: Sort;
  widthOf: (id: string) => number;
  onSort: (by: SortBy) => void;
  onResizeStart: (id: string, width: number, e: PointerEvent) => void;
  /** Hierarchy: the Summary name lines up with the titles behind a parent's caret. */
  tree?: boolean;
  /** An ungrouped view: the checkbox in the rows' checkbox column that picks or unpicks every row. */
  pickAll?: { picks: GroupPicks; onTogglePicks: () => void };
}

/** The sticky header row: every sortable name sorts on click, every column but Summary resizes at its edge. */
export function TableHeader({ listId, columns, sort, widthOf, onSort, onResizeStart, tree = false, pickAll }: Props) {
  const { data: catalog = [] } = useColumnsCatalogue(listId);
  const name = (id: string) => FIXED_NAMES[id] ?? catalog.find((c) => c.id === id)?.name ?? id;

  const className = ["table-header", tree && "table-header--tree", pickAll && "table-header--picks"].filter(Boolean).join(" ");
  return (
    <div className={className} role="row">
      {pickAll && (
        // A div, so the Key cell stays the first span (table.css gives it the key's inset).
        <div className="table-row__pick">
          <GroupBox group={null} picks={pickAll.picks} onToggle={pickAll.onTogglePicks} />
        </div>
      )}
      {columns.map((id) => {
        const by = sortByOf(id);
        const sorted = by !== null && sort.by === by;
        const flexible = id === "summary";
        const width = widthOf(id);
        return (
          <span
            key={id}
            role="columnheader"
            className={flexible ? "table-header__cell table-header__cell--flex" : "table-header__cell"}
            style={flexible ? undefined : { width }}
            aria-sort={sorted ? (sort.descending ? "descending" : "ascending") : undefined}
          >
            {by ? (
              <button type="button" className="table-header__sort" onClick={() => onSort(by)}>
                {name(id)}
                {sorted && <span aria-hidden="true">{sort.descending ? "↓" : "↑"}</span>}
              </button>
            ) : (
              <span className="table-header__name">{name(id)}</span>
            )}
            {!flexible && (
              <span
                className="table-header__resize"
                role="separator"
                aria-orientation="vertical"
                aria-label={`Resize ${name(id)}`}
                onPointerDown={(e) => onResizeStart(id, width, e)}
              />
            )}
          </span>
        );
      })}
    </div>
  );
}
