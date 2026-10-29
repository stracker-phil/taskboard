import { useEffect, useRef, useState } from "react";

/** The issue under the pointer while ⌥ is held: anything marked `data-issue-id` (rows, cards, related issues). */
export interface PeekTarget {
  id: string;
  rect: DOMRect;
}

/** How long the pointer rests on an issue with ⌥ held before the peek shows. */
export const PEEK_DELAY_MS = 150;

const issueElement = (node: EventTarget | null) => (node instanceof Element ? node.closest<HTMLElement>("[data-issue-id]") : null);

/** Follows ⌥ and the pointer; the peek shows after a short rest and hides when ⌥ is released or the pointer
 *  leaves the issue. */
export function useHoverPeek(): PeekTarget | null {
  const [target, setTarget] = useState<PeekTarget | null>(null);
  const shown = useRef<string | null>(null);
  useEffect(() => {
    let alt = false;
    let over: HTMLElement | null = null;
    let pending: { id: string; timer: ReturnType<typeof setTimeout> } | null = null;
    const cancel = () => {
      if (pending) clearTimeout(pending.timer);
      pending = null;
    };
    const update = () => {
      const id = alt ? over?.dataset.issueId : undefined;
      if (!id || !over) {
        cancel();
        shown.current = null;
        setTarget(null);
        return;
      }
      if (id === shown.current || id === pending?.id) return;
      cancel();
      const element = over;
      pending = {
        id,
        timer: setTimeout(() => {
          pending = null;
          shown.current = id;
          setTarget({ id, rect: element.getBoundingClientRect() });
        }, PEEK_DELAY_MS),
      };
    };
    const onPointer = (e: PointerEvent) => {
      alt = e.altKey;
      over = issueElement(e.target);
      update();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Alt") return;
      alt = e.type === "keydown";
      update();
    };
    const reset = () => {
      alt = false;
      update();
    };
    window.addEventListener("pointerover", onPointer);
    window.addEventListener("pointermove", onPointer);
    window.addEventListener("keydown", onKey);
    window.addEventListener("keyup", onKey);
    window.addEventListener("blur", reset);
    return () => {
      cancel();
      window.removeEventListener("pointerover", onPointer);
      window.removeEventListener("pointermove", onPointer);
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("keyup", onKey);
      window.removeEventListener("blur", reset);
    };
  }, []);
  return target;
}
