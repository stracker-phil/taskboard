import type { Drag } from "./useIssueDrag";

/** The dragged issue's key and summary at the pointer. */
export function DragGhost({ drag }: { drag: Drag | null }) {
  if (!drag) return null;
  return (
    <div className="drag-ghost" style={{ left: drag.at.x + 12, top: drag.at.y + 12 }} aria-hidden="true">
      <span className="drag-ghost__key">{drag.issue.key}</span>
      <span className="drag-ghost__summary">{drag.issue.summary}</span>
    </div>
  );
}
