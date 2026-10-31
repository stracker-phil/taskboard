import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { api, type DropTargets, type GroupBy } from "../api";
import { attempt } from "../state/requests";
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
  at: Point;
  /** Until Rust answers, no group takes the drop. */
  targets: DropTargets | null;
}

export type DropState = "ok" | "no" | "home";

/** Rows and cards dragged onto another group change the field the view groups by (`docs/writes.md`); drop zones
 *  carry `data-drop-key`. */
export function useIssueDrag(groupBy: GroupBy, openInJira: ((key: string) => void) | null) {
  const changesAllowed = useJiraChanges();
  const [drag, setDrag] = useState<Drag | null>(null);
  const latest = useRef<Drag | null>(null);
  const update = (next: Drag | null) => {
    latest.current = next;
    setDrag(next);
  };
  useDraggingClass(drag !== null);

  const onPointerDown = (e: ReactPointerEvent, issue: Dragged) => {
    if (!changesAllowed || e.button !== 0 || e.metaKey || e.ctrlKey || e.altKey || e.shiftKey) return;
    if ((e.target as HTMLElement).closest("button, a, input, textarea")) return;
    followPointer(
      { x: e.clientX, y: e.clientY },
      {
        onStart: () => {
          update({ issue, at: { x: e.clientX, y: e.clientY }, targets: null });
          void attempt(api.issueDropTargets(issue.id, groupBy), "Could not ask Jira where the issue may go").then((targets) => {
            if (latest.current?.issue.id === issue.id) update({ ...latest.current, targets });
          });
        },
        onMove: (at) => latest.current && update({ ...latest.current, at }),
        onDrop: (at) => {
          const key = dropKeyAt(at);
          const finished = latest.current;
          if (finished && key !== null && dropState(finished, key) === "ok") drop(issue, groupBy, key, at, openInJira);
        },
        onEnd: () => update(null),
      },
    );
  };

  return { drag, onPointerDown, dropState: (key: string) => (drag ? dropState(drag, key) : null) };
}

/** The issue's own group is home; a group takes it when Rust listed it (or every group). */
function dropState(drag: Drag, key: string): DropState {
  if (key === drag.issue.fromKey) return "home";
  const targets = drag.targets;
  return targets && (targets.any || targets.keys.includes(key)) ? "ok" : "no";
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
