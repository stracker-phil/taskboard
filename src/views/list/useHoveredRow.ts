import { type PointerEvent, useState } from "react";

/** The issue row under the pointer (anything marked `data-issue-id`), and the grid's handlers for it. */
export function useHoveredRow() {
  const [hovered, setHovered] = useState<string | null>(null);
  return {
    hovered,
    onPointerOver: (e: PointerEvent) => {
      const row = e.target instanceof Element ? e.target.closest<HTMLElement>("[data-issue-id]") : null;
      setHovered(row?.dataset.issueId ?? null);
    },
    onPointerLeave: () => setHovered(null),
  };
}
