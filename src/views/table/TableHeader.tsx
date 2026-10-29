import type { PointerEvent } from "react";
import type { Sort, SortBy } from "../../api";
import { useColumnsCatalogue } from "../../queries/views";
import { sortByOf } from "./sortable";

const FIXED_NAMES: Record<string, string> = { key: "Key", summary: "Summary", status: "Status" };

interface Props {
  listId: number;
  columns: string[];
  sort: Sort;
  widthOf: (id: string) => number;
  onSort: (by: SortBy) => void;
  onResizeStart: (id: string, width: number, e: PointerEvent) => void;
}

/** The sticky header row: every sortable name sorts on click, every column but Summary resizes at its edge. */
export function TableHeader({ listId, columns, sort, widthOf, onSort, onResizeStart }: Props) {
  const { data: catalogue = [] } = useColumnsCatalogue(listId);
  const name = (id: string) => FIXED_NAMES[id] ?? catalogue.find((c) => c.id === id)?.name ?? id;

  return (
    <div className="table-header" role="row">
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
