import { useCallback, useEffect, useRef, useState } from "react";
import { motionMs } from "../../lib/motion";

/** `--ease-in-out` as a function of progress 0…1. */
function easeInOut(p: number): number {
  return p < 0.5 ? 2 * p * p : 1 - (-2 * p + 2) ** 2 / 2;
}

/** Animates a group's rows open or closed. A virtualised group isn't one element, so instead of a CSS height
 *  this drives a factor 0…1 per group that scales its rows' heights and opacity; only groups in motion have
 *  one. With no motion time (reduced motion, tests) nothing animates. */
export function useGroupMotion() {
  const [factors, setFactors] = useState<Record<string, number>>({});
  const frames = useRef(new Map<string, number>());

  useEffect(() => {
    const running = frames.current;
    return () => running.forEach((id) => cancelAnimationFrame(id));
  }, []);

  const animate = useCallback((key: string, opening: boolean) => {
    const ms = motionMs("--motion-slow");
    const previous = frames.current.get(key);
    if (previous !== undefined) cancelAnimationFrame(previous);
    if (ms === 0) {
      frames.current.delete(key);
      setFactors(({ [key]: _done, ...rest }) => rest);
      return;
    }
    const [from, to] = opening ? [0, 1] : [1, 0];
    setFactors((all) => ({ ...all, [key]: from }));
    const start = performance.now();
    const step = (now: number) => {
      const progress = Math.min(1, (now - start) / ms);
      if (progress < 1) {
        setFactors((all) => ({ ...all, [key]: from + (to - from) * easeInOut(progress) }));
        frames.current.set(key, requestAnimationFrame(step));
      } else {
        frames.current.delete(key);
        setFactors(({ [key]: _done, ...rest }) => rest);
      }
    };
    frames.current.set(key, requestAnimationFrame(step));
  }, []);

  return { factors, animate };
}
