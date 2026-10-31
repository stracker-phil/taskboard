export interface Point {
  x: number;
  y: number;
}

interface Gesture {
  /** The pointer moved far enough from the press: a drag. */
  onStart: () => void;
  onMove: (at: Point) => void;
  /** Released after a drag started; before `onEnd`. */
  onDrop: (at: Point) => void;
  /** The gesture ended (released, esc, or no drag at all). */
  onEnd: () => void;
}

/** How far the pointer moves before a press becomes a drag. */
const DRAG_AFTER_PX = 6;

/** Follows a press until the pointer is released; a press that barely moves stays a click, esc cancels a drag, and
 *  the click that ends a drag reaches nothing. */
export function followPointer(press: Point, gesture: Gesture) {
  let dragging = false;
  const move = (e: PointerEvent) => {
    if (!dragging) {
      if (Math.hypot(e.clientX - press.x, e.clientY - press.y) < DRAG_AFTER_PX) return;
      dragging = true;
      gesture.onStart();
    }
    gesture.onMove({ x: e.clientX, y: e.clientY });
  };
  const up = (e: PointerEvent) => {
    if (dragging) {
      swallowNextClick();
      gesture.onDrop({ x: e.clientX, y: e.clientY });
    }
    stop();
  };
  const key = (e: KeyboardEvent) => {
    if (e.key !== "Escape" || !dragging) return;
    e.preventDefault();
    e.stopPropagation();
    stop();
  };
  const stop = () => {
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", up);
    window.removeEventListener("keydown", key, true);
    gesture.onEnd();
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", up);
  window.addEventListener("keydown", key, true);
}

function swallowNextClick() {
  const swallow = (e: MouseEvent) => {
    e.stopPropagation();
    e.preventDefault();
  };
  window.addEventListener("click", swallow, { capture: true, once: true });
  // A release outside any element gets no click; the listener mustn't eat a later one.
  setTimeout(() => window.removeEventListener("click", swallow, true), 0);
}
