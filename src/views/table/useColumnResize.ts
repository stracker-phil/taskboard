import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { MIN_COLUMN_WIDTH } from "./widths";

/** Dragging a header's edge: the width follows the pointer and is committed once on release. */
export function useColumnResize(onCommit: (id: string, width: number) => void) {
  const [live, setLive] = useState<{ id: string; width: number } | null>(null);
  const stopRef = useRef<(() => void) | null>(null);

  useEffect(() => () => stopRef.current?.(), []);

  const start = (id: string, startWidth: number, e: ReactPointerEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const startX = e.clientX;
    let width = startWidth;
    const move = (ev: PointerEvent) => {
      width = Math.max(MIN_COLUMN_WIDTH, Math.round(startWidth + ev.clientX - startX));
      setLive({ id, width });
    };
    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", release);
      stopRef.current = null;
    };
    const release = () => {
      stop();
      setLive(null);
      if (width !== startWidth) onCommit(id, width);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", release);
    stopRef.current = stop;
  };

  return { liveWidth: (id: string) => (live?.id === id ? live.width : undefined), start };
}
