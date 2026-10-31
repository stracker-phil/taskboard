import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent, type RefObject } from "react";
import { api, type DropTargets, type GroupBy, type ViewGroup } from "../api";
import { attempt } from "../state/requests";
import { followDraggedCopy } from "./draggedCopy";
import { followPointer, type Point } from "./followPointer";
import { showOutcome } from "./runWrite";
import { openMoveMenu } from "./statusMenu";
import { useJiraChanges } from "./useJiraChanges";

/** The issue a drag carries, and the group it comes from. */
interface Dragged {
  id: string;
  key: string;
  summary: string;
  fromKey: string;
}

export interface Drag {
  issue: Dragged;
  /** Until Rust answers, no group takes the drop. */
  targets: DropTargets | null;
}

/** Which way the view scrolls when the pointer nears its edge. */
export type ScrollAxis = "x" | "y";

/** Rows and cards dragged onto a group's drop zone change the field the view groups by (`docs/writes.md`); zones
 *  carry `data-drop-key`. While a drag lasts the row or card stays in place, dimmed, and a copy of it follows the
 *  pointer; near the view's edge the view scrolls. */
export function useIssueDrag(
  groupBy: GroupBy,
  groups: ViewGroup[],
  openInJira: ((key: string) => void) | null,
  host: RefObject<HTMLElement | null>,
  axis: ScrollAxis,
) {
  const changesAllowed = useJiraChanges();
  const [drag, setDrag] = useState<Drag | null>(null);
  const latest = useRef<Drag | null>(null);
  const update = (next: Drag | null) => {
    latest.current = next;
    setDrag(next);
  };
  useDraggingClass(drag !== null);

  /** `element`: the row or card as it shows, which the copy repeats. */
  const onPointerDown = (e: ReactPointerEvent, issue: Dragged, element: HTMLElement) => {
    if (!changesAllowed || e.button !== 0 || e.metaKey || e.ctrlKey || e.altKey || e.shiftKey) return;
    if ((e.target as HTMLElement).closest("button, a, input, textarea")) return;
    const press = { x: e.clientX, y: e.clientY };
    let copy: ReturnType<typeof followDraggedCopy> | null = null;
    followPointer(press, {
      onStart: () => {
        const view = host.current;
        if (view) copy = followDraggedCopy(element, press, view, axis);
        update({ issue, targets: null });
        void attempt(api.issueDropTargets(issue.id, groupBy, issue.fromKey, groups), "Could not ask Jira where the issue may go").then((targets) => {
          if (latest.current?.issue.id === issue.id) update({ ...latest.current, targets });
        });
      },
      onMove: (at) => {
        copy?.moveTo(at);
        const key = dropKeyAt(at);
        const target = key === null ? null : (latest.current?.targets?.targets.find((t) => t.key === key) ?? null);
        copy?.preview(target, groupBy === "status");
      },
      onDrop: (at) => {
        const key = dropKeyAt(at);
        const finished = latest.current;
        if (finished && key !== null && takesDrop(finished, key)) drop(issue, groupBy, key, at, openInJira);
      },
      onEnd: () => {
        copy?.remove();
        update(null);
      },
    });
  };

  return { drag, onPointerDown };
}

function takesDrop(drag: Drag, key: string): boolean {
  return drag.targets?.targets.some((target) => target.key === key) ?? false;
}

function dropKeyAt(at: Point): string | null {
  return document.elementFromPoint(at.x, at.y)?.closest<HTMLElement>("[data-drop-key]")?.dataset.dropKey ?? null;
}

/** A done drop toasts its outcome; one onto a status type several statuses fit asks which. */
function drop(issue: Dragged, groupBy: GroupBy, key: string, at: Point, openInJira: ((key: string) => void) | null) {
  void attempt(api.issueDrop(issue.id, groupBy, key), "Could not move the issue").then((dropped) => {
    if (dropped?.kind === "done") showOutcome(issue.id, dropped.outcome);
    else if (dropped?.kind === "choose") openMoveMenu(issue, dropped.moves, at, openInJira);
  });
}

/** The grabbing cursor everywhere, and no text selection, while an issue is dragged. */
function useDraggingClass(dragging: boolean) {
  useEffect(() => {
    if (!dragging) return;
    document.body.classList.add("is-dragging-issue");
    return () => document.body.classList.remove("is-dragging-issue");
  }, [dragging]);
}
