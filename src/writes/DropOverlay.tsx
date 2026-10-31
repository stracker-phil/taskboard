import { useLayoutEffect, useState, type CSSProperties, type RefObject } from "react";
import { createPortal } from "react-dom";
import type { GroupMark } from "../api";
import { Mark } from "../views/list/GroupHeader";
import type { Drag } from "./useIssueDrag";

interface Props {
  drag: Drag | null;
  /** The view's own area, which the tiles cover while a drag lasts. */
  host: RefObject<HTMLElement | null>;
}

/** While an issue is dragged, the view's rows give way to one tile per place it may go, all the same size, each
 *  saying what a drop there does (`docs/writes.md`). */
export function DropOverlay({ drag, host }: Props) {
  const area = useHostArea(drag !== null, host);
  if (!drag || !area) return null;
  const { issue, targets } = drag;
  const tiles = targets?.targets ?? [];
  const refused = targets?.refused ?? [];

  return createPortal(
    <div className="drop-overlay" style={area} role="list" aria-label={`Places to move ${issue.key} to`}>
      <p className="drop-overlay__caption">
        Drop {issue.key} on a tile to change it · <kbd>esc</kbd> cancels
      </p>
      {targets === null && <p className="drop-overlay__note">Asking Jira where {issue.key} can go…</p>}
      {targets !== null && tiles.length === 0 && <p className="drop-overlay__note">{targets.note ?? `Jira doesn't let ${issue.key} move anywhere from here.`}</p>}
      {tiles.length > 0 && (
        <div className="drop-overlay__tiles">
          {tiles.map((target) => (
            <div key={target.key} role="listitem" className={`drop-tile ${colorClass(target.mark)}`} data-drop-key={target.key}>
              <TileMark mark={target.mark} />
              <span className="drop-tile__action">{target.action}</span>
            </div>
          ))}
        </div>
      )}
      {tiles.length > 0 && refused.length > 0 && <p className="drop-overlay__note">Not allowed here: {refused.join(", ")}</p>}
    </div>,
    document.body,
  );
}

function TileMark({ mark }: { mark: GroupMark | null }) {
  if (mark?.kind === "status") return <span className={`status-square status-square--${mark.style}`} aria-hidden="true" />;
  return <Mark mark={mark} />;
}

/** The outline takes the group's color: the status style, the person's avatar, the priority's flag. */
function colorClass(mark: GroupMark | null): string {
  switch (mark?.kind) {
    case "status":
      return `drop-tile--status-${mark.style}`;
    case "person":
      return `drop-tile--avatar-${mark.person.color}`;
    case "priority":
      return `drop-tile--priority-${Math.min(Math.max(mark.level, 1), 4)}`;
    default:
      return "drop-tile--plain";
  }
}

/** Where the host sits in the window when the drag starts. */
function useHostArea(dragging: boolean, host: RefObject<HTMLElement | null>): CSSProperties | null {
  const [area, setArea] = useState<CSSProperties | null>(null);
  useLayoutEffect(() => {
    const rect = dragging ? host.current?.getBoundingClientRect() : undefined;
    setArea(rect ? { left: rect.left, top: rect.top, width: rect.width, height: rect.height } : null);
  }, [dragging, host]);
  return area;
}
