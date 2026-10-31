import { type ReactNode, type RefObject, useLayoutEffect, useRef } from "react";
import type { Group } from "../../api";
import { colorClass, DropZone, zoneOf } from "../../writes/DropZone";
import type { Drag } from "../../writes/useIssueDrag";
import { dragGroups } from "./dragGroups";
import type { Item } from "./flatten";

/** Rows each group shows while an issue is dragged; the card keeps room for one more line ("+N more"). */
const SHOWN_ROWS = 2;

type RowItem = Extract<Item, { kind: "row" }>;

interface Props {
  drag: Drag;
  /** The view flattened with every group open. */
  items: Item[];
  /** "table": rows edge to edge, as the Table draws them; "list": each group a card. */
  variant: "list" | "table";
  header: (group: Group, tab: boolean) => ReactNode;
  row: (item: RowItem) => ReactNode;
}

/** A List or Table while an issue is dragged: every group, closed ones too, with the same few rows, so each group is
 *  an equally easy target; the group's rows are its drop zone. The dragged row stays in its group, dimmed. */
export function DragList({ drag, items, variant, header, row }: Props) {
  return (
    <div className={`drag-list drag-list--${variant}`}>
      {dragGroups(items, drag.issue.id, SHOWN_ROWS).map(({ group, tab, first, rows, more }) => {
        const zone = zoneOf(drag, group.key);
        const cardClass = ["drag-card", tab && "drag-card--tabbed", zone && `drag-card--${zone.state} ${colorClass(zone.target?.mark ?? null)}`];
        return (
          <section key={group.key} className={["drag-list__group", !first && "drag-list__group--gap"].filter(Boolean).join(" ")} data-drag-group={group.key}>
            {header(group, tab)}
            <div className={cardClass.filter(Boolean).join(" ")}>
              {rows.map((item) => (
                <div key={item.row.id} data-drag-source={item.row.id === drag.issue.id ? "" : undefined}>
                  {row(item)}
                </div>
              ))}
              <div className="drag-card__more">{more > 0 ? `+${more} more` : rows.length === 0 ? "No issues" : ""}</div>
              {zone && <DropZone zone={zone} />}
            </div>
          </section>
        );
      })}
    </div>
  );
}

/** Brings the dragged row's group into view when the drag layout opens, and puts the view back where it was after. */
export function useDragScroll(scrollRef: RefObject<HTMLElement | null>, fromKey: string | null) {
  const before = useRef(0);
  useLayoutEffect(() => {
    const view = scrollRef.current;
    if (!view || fromKey === null) return;
    before.current = view.scrollTop;
    view.querySelector<HTMLElement>(`[data-drag-group="${CSS.escape(fromKey)}"]`)?.scrollIntoView?.({ block: "center" });
    return () => {
      view.scrollTop = before.current;
    };
  }, [scrollRef, fromKey]);
}
