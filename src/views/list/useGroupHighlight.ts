import { type FocusEvent, type PointerEvent, useState } from "react";

/** Items carry `data-group`; the highlight reads it from the event's target. */
function groupAt(target: EventTarget | null): string | null {
  return target instanceof Element ? (target.closest<HTMLElement>("[data-group]")?.dataset.group ?? null) : null;
}

/** Which group has the pointer or the focus (its header shows the chevron), and the grid's handlers for it. */
export function useGroupHighlight() {
  const [hovered, setHovered] = useState<string | null>(null);
  const [focused, setFocused] = useState<string | null>(null);
  return {
    isActive: (group: string) => hovered === group || focused === group,
    handlers: {
      onPointerOver: (e: PointerEvent) => setHovered(groupAt(e.target)),
      onPointerLeave: () => setHovered(null),
      onFocus: (e: FocusEvent) => setFocused(groupAt(e.target)),
      onBlur: () => setFocused(null),
    },
  };
}
