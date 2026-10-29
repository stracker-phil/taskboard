import { useRef, useState, type KeyboardEvent, type PointerEvent } from "react";
import { useStore } from "../state/store";

export const ACTIVITY_WIDTH = { default: 400, min: 300, max: 720 } as const;
const STEP = 16;

const clamp = (width: number) => Math.round(Math.min(ACTIVITY_WIDTH.max, Math.max(ACTIVITY_WIDTH.min, width)));

/** The activity column's width and its left-edge handle: drag or ←/→; the width is saved for every issue. */
export function useActivityWidth() {
  const saved = useStore((s) => s.activityWidth);
  const save = useStore((s) => s.setActivityWidth);
  const [dragging, setDragging] = useState<number | null>(null);
  const start = useRef({ x: 0, width: 0 });
  const width = dragging ?? clamp(saved ?? ACTIVITY_WIDTH.default);

  const handle = {
    onPointerDown: (e: PointerEvent<HTMLElement>) => {
      e.currentTarget.setPointerCapture(e.pointerId);
      start.current = { x: e.clientX, width };
      setDragging(width);
    },
    // The handle is on the column's left edge: moving left widens it.
    onPointerMove: (e: PointerEvent<HTMLElement>) => {
      if (dragging !== null) setDragging(clamp(start.current.width + start.current.x - e.clientX));
    },
    onPointerUp: () => {
      if (dragging !== null) save(dragging);
      setDragging(null);
    },
    onKeyDown: (e: KeyboardEvent<HTMLElement>) => {
      const step = e.key === "ArrowLeft" ? STEP : e.key === "ArrowRight" ? -STEP : 0;
      if (step === 0) return;
      e.preventDefault();
      save(clamp(width + step));
    },
  };
  return { width, dragging: dragging !== null, handle };
}
