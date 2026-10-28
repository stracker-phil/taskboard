import { useEffect, useState } from "react";

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
