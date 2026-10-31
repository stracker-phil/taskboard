import { useLayoutEffect, useState, type CSSProperties, type RefObject } from "react";
import type { DropTarget, GroupMark, ViewGroup } from "../api";
import { Mark } from "../views/list/GroupHeader";
import type { Drag } from "./useIssueDrag";

/** A group while an issue is dragged: it takes the drop, can't, or Jira hasn't answered yet. */
export interface Zone {
  state: "ok" | "no" | "wait";
  target: DropTarget | null;
}

/** How the group `key` shows during the drag; null without a drag and for the issue's own group, which stays as it
 *  is so the issue can go back there. */
export function zoneOf(drag: Drag | null, key: string): Zone | null {
  if (!drag || key === drag.issue.fromKey) return null;
  if (!drag.targets) return { state: "wait", target: null };
  const target = drag.targets.targets.find((t) => t.key === key) ?? null;
  return { state: target ? "ok" : "no", target };
}

/** A group's drop zone over its rows or cards: outlined and tinted in the group's color with the action across
 *  it, or faded where the issue can't go. Only a zone that takes the drop carries `data-drop-key`. */
export function DropZone({ zone, style, className = "" }: { zone: Zone; style?: CSSProperties; className?: string }) {
  const { state, target } = zone;
  return (
    <div className={`drop-zone drop-zone--${state} ${colorClass(target?.mark ?? null)} ${className}`} style={style} data-drop-key={target?.key}>
      {target && (
        <span className="drop-zone__action">
          <TargetMark mark={target.mark} />
          {target.action}
        </span>
      )}
    </div>
  );
}

/** Places the view has no group for (a teammate without issues here), as chips along the view's bottom edge. */
export function DropTray({ drag, groups, host }: { drag: Drag | null; groups: ViewGroup[]; host: RefObject<HTMLElement | null> }) {
  const area = useBottomEdge(drag !== null, host);
  const extra = drag?.targets?.targets.filter((t) => t.key !== drag.issue.fromKey && !groups.some((g) => g.key === t.key)) ?? [];
  if (!area || extra.length === 0) return null;
  return (
    <div className="drop-tray" style={area}>
      <span className="drop-tray__caption">Also</span>
      {extra.map((target) => (
        <div key={target.key} className={`drop-chip ${colorClass(target.mark)}`} data-drop-key={target.key}>
          <TargetMark mark={target.mark} />
          {target.action}
        </div>
      ))}
    </div>
  );
}

function TargetMark({ mark }: { mark: GroupMark | null }) {
  if (mark?.kind === "status") return <span className={`status-square status-square--${mark.style}`} aria-hidden="true" />;
  return <Mark mark={mark} />;
}

/** The group's color: the status style, the person's avatar, the priority's flag. */
export function colorClass(mark: GroupMark | null): string {
  switch (mark?.kind) {
    case "status":
      return `drop-color--status-${mark.style}`;
    case "person":
      return `drop-color--avatar-${mark.person.color}`;
    case "priority":
      return `drop-color--priority-${Math.min(Math.max(mark.level, 1), 4)}`;
    default:
      return "drop-color--plain";
  }
}

/** The view's bottom edge in the window when the drag starts. */
function useBottomEdge(dragging: boolean, host: RefObject<HTMLElement | null>): CSSProperties | null {
  const [area, setArea] = useState<CSSProperties | null>(null);
  useLayoutEffect(() => {
    const rect = dragging ? host.current?.getBoundingClientRect() : undefined;
    setArea(rect ? { left: rect.left + 16, width: rect.width - 32, bottom: window.innerHeight - rect.bottom + 16 } : null);
  }, [dragging, host]);
  return area;
}
