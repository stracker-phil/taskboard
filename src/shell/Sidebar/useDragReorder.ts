import { useRef, useState, type MouseEvent, type PointerEvent } from "react";

/** How far the pointer moves before a press becomes a drag. */
const THRESHOLD = 4;

interface Drag {
  from: number;
  /** The gap the item would land in: 0 above the first item, n below the last. */
  gap: number;
  /** Where the drop line sits, from the container's top. */
  lineY: number;
}

/** The container's direct children marked as items, in order. */
function items(container: HTMLElement): HTMLElement[] {
  return [...container.querySelectorAll<HTMLElement>(":scope > [data-drag-index]")];
}

function gapAt(container: HTMLElement, y: number): Omit<Drag, "from"> {
  const list = items(container);
  // In content coordinates, so the line stays put in a scrolled list.
  const top = container.getBoundingClientRect().top - container.scrollTop;
  const index = list.findIndex((el) => {
    const rect = el.getBoundingClientRect();
    return y < rect.top + rect.height / 2;
  });
  if (index === -1) {
    const last = list[list.length - 1]?.getBoundingClientRect();
    return { gap: list.length, lineY: last ? last.bottom - top : 0 };
  }
  return { gap: index, lineY: list[index].getBoundingClientRect().top - top };
}

/**
 * Drag to reorder a vertical list with pointer events. The container holds the items as direct children
 * (`item(i)`); a press on an item's `handle(i)` that moves starts the drag, a drop line shows the gap, and the
 * release calls `onMove(from, to)` with the item's new index. The click that ends a drag is swallowed.
 */
export function useDragReorder<T extends HTMLElement>(onMove: (from: number, to: number) => void) {
  const containerRef = useRef<T>(null);
  const press = useRef<{ index: number; y: number } | null>(null);
  const justDragged = useRef(false);
  const [drag, setDrag] = useState<Drag | null>(null);

  const end = () => {
    press.current = null;
    setDrag(null);
  };

  const handle = (index: number) => ({
    onPointerDown: (e: PointerEvent<HTMLElement>) => {
      if (e.button !== 0) return;
      press.current = { index, y: e.clientY };
      justDragged.current = false;
    },
    onPointerMove: (e: PointerEvent<HTMLElement>) => {
      const container = containerRef.current;
      if (!press.current || !container) return;
      // Released outside the row: the press is over.
      if (!(e.buttons & 1)) return end();
      if (!drag) {
        if (Math.abs(e.clientY - press.current.y) < THRESHOLD) return;
        // Captured only once it drags: a capture from the press would retarget a plain click to the row.
        e.currentTarget.setPointerCapture?.(e.pointerId);
      }
      setDrag({ from: press.current.index, ...gapAt(container, e.clientY) });
    },
    onPointerUp: () => {
      if (drag) {
        justDragged.current = true;
        const to = drag.gap > drag.from ? drag.gap - 1 : drag.gap;
        if (to !== drag.from) onMove(drag.from, to);
      }
      end();
    },
    onPointerCancel: end,
    onClickCapture: (e: MouseEvent) => {
      if (!justDragged.current) return;
      justDragged.current = false;
      e.preventDefault();
      e.stopPropagation();
    },
  });

  const item = (index: number) => ({ "data-drag-index": index, "data-dragging": drag?.from === index || undefined });

  return { containerRef, item, handle, dropLineY: drag ? drag.lineY : null };
}
