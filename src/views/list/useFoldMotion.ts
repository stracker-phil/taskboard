import { useEffect, useRef, useState } from "react";
import { motionMs } from "../../lib/motion";

/** `--ease-in-out` as a function of progress 0…1. */
function easeInOut(p: number): number {
  return p < 0.5 ? 2 * p * p : 1 - (-2 * p + 2) ** 2 / 2;
}

/** How far the view has folded for a drag (0…1): it folds while `folded` is true and unfolds after, at the
 *  expand/collapse speed (`--motion-slow`); instant under reduced motion. `moving` is true until it has unfolded. */
export function useFoldMotion(folded: boolean): { progress: number; moving: boolean } {
  const [progress, setProgress] = useState(0);
  const current = useRef(0);
  useEffect(() => {
    const target = folded ? 1 : 0;
    const ms = motionMs("--motion-slow");
    const from = current.current;
    if (from === target) return;
    if (ms === 0) {
      current.current = target;
      setProgress(target);
      return;
    }
    const start = performance.now();
    const duration = ms * Math.abs(target - from);
    let frame = requestAnimationFrame(function step(now) {
      const p = Math.min(1, (now - start) / duration);
      current.current = from + (target - from) * easeInOut(p);
      setProgress(current.current);
      if (p < 1) frame = requestAnimationFrame(step);
    });
    return () => cancelAnimationFrame(frame);
  }, [folded]);
  return { progress, moving: folded || progress > 0 };
}
