import { type RefObject, useEffect, useLayoutEffect, useRef, useState } from "react";

export type MotionToken = "--motion-fast" | "--motion-base" | "--motion-slow";

/** A motion token's duration in ms, as the stylesheet sets it (0 under reduced motion, or without styles). */
export function motionMs(token: MotionToken): number {
  const value = getComputedStyle(document.documentElement).getPropertyValue(token).trim();
  const ms = Number.parseFloat(value);
  if (!Number.isFinite(ms)) return 0;
  return value.endsWith("ms") ? ms : ms * 1000;
}

export type Presence = "open" | "leaving";

/** Keeps something mounted while it fades out: `mounted` stays true for the exit time after `shown` turns
 *  false, with `state` "leaving" so CSS can play the exit. Showing it again during the exit cancels it. */
export function usePresence(shown: boolean, token: MotionToken = "--motion-base"): { mounted: boolean; state: Presence } {
  const [leaving, setLeaving] = useState(false);
  const [wasShown, setWasShown] = useState(shown);
  if (shown !== wasShown) {
    setWasShown(shown);
    setLeaving(!shown && motionMs(token) > 0);
  }
  useEffect(() => {
    if (!leaving) return;
    const timer = setTimeout(() => setLeaving(false), motionMs(token));
    return () => clearTimeout(timer);
  }, [leaving, token]);
  return { mounted: shown || leaving, state: shown ? "open" : "leaving" };
}

/** Where a row of controls had each control, and how wide it was then. */
interface Places {
  width: number;
  lefts: WeakMap<Element, number>;
}

/** The children of `row` glide to a new place instead of jumping, and a new child fades in (docs/ux.md → Stable
 *  controls). Only while the row keeps its width: when the window resizes, they follow at once. When `scope`
 *  changes (another view) the row shows its new state at once, also without its controls' CSS transitions. */
export function useGlide(row: RefObject<HTMLElement | null>, scope?: unknown) {
  const before = useRef<Places | null>(null);
  const shownScope = useRef(scope);
  useLayoutEffect(() => {
    const element = row.current;
    if (!element) return;
    const switched = shownScope.current !== scope;
    shownScope.current = scope;
    if (switched) instantly(element);
    const now: Places = { width: element.clientWidth, lefts: new WeakMap() };
    const previous = before.current;
    const ms = motionMs("--motion-slow");
    const animates = !switched && previous !== null && previous.width === now.width && ms > 0;
    const easing = getComputedStyle(document.documentElement).getPropertyValue("--ease-out").trim() || "ease-out";
    for (const child of element.children) {
      if (!(child instanceof HTMLElement)) continue;
      const left = child.offsetLeft;
      now.lefts.set(child, left);
      if (!animates || typeof child.animate !== "function") continue;
      const was = previous.lefts.get(child);
      if (was === undefined) child.animate([{ opacity: 0, transform: "scale(0.92)" }, { opacity: 1, transform: "none" }], { duration: ms, easing });
      else if (was !== left) child.animate([{ transform: `translateX(${was - left}px)` }, { transform: "none" }], { duration: ms, easing });
    }
    before.current = now;
  });
}

/** Turns off `element`'s transitions until the frame with its new state is drawn. */
function instantly(element: HTMLElement) {
  element.classList.add("motion-off");
  requestAnimationFrame(() => requestAnimationFrame(() => element.classList.remove("motion-off")));
}
