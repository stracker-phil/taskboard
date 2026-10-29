import { useVirtualizer } from "@tanstack/react-virtual";
import { memo, useEffect, useRef } from "react";
import type { Row } from "../../api";
import type { RowActionHandlers } from "../list/RowActions";
import { BoardCard } from "./BoardCard";

interface Props {
  ids: string[];
  rows: Record<string, Row>;
  fields: string[];
  inlineLabels: boolean;
  /** The keyboard's current card when it is in this column. */
  selected: string | null;
  onOpen: (id: string) => void;
  actions: RowActionHandlers;
}

const CARD_ESTIMATE = 110;
const CARD_GAP = 8;

/** A column's cards, virtualized: a Space with thousands of issues mounts only the cards on screen. */
export const BoardColumnCards = memo(function BoardColumnCards({ ids, rows, fields, inlineLabels, selected, onOpen, actions }: Props) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: ids.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => CARD_ESTIMATE,
    getItemKey: (index) => ids[index],
    gap: CARD_GAP,
    paddingEnd: 4,
    overscan: 4,
    // Scrolling to the keyboard's card runs from an effect, where a synchronous render isn't allowed.
    useFlushSync: false,
  });

  useEffect(() => {
    const index = selected ? ids.indexOf(selected) : -1;
    if (index >= 0) virtualizer.scrollToIndex(index, { align: "auto" });
  }, [ids, selected, virtualizer]);

  return (
    <div className="board-column__cards" ref={scrollRef}>
      <div className="board-column__track" style={{ height: virtualizer.getTotalSize() }}>
        {virtualizer.getVirtualItems().map((virtual) => {
          const row = rows[ids[virtual.index]];
          return row ? (
            <div
              key={virtual.key}
              className="board-column__slot"
              data-index={virtual.index}
              ref={virtualizer.measureElement}
              style={{ transform: `translateY(${virtual.start}px)` }}
            >
              <BoardCard row={row} fields={fields} inlineLabels={inlineLabels} selected={row.id === selected} onOpen={onOpen} actions={actions} />
            </div>
          ) : null;
        })}
      </div>
    </div>
  );
});
