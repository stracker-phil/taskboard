import { type RefObject, useLayoutEffect, useRef, useState } from "react";
import { type Collapsible, overflowing } from "./fit";

/** The toolbar's horizontal padding (24 each side). */
const PADDING = 48;

/** Watches the toolbar's width and returns the items that don't fit. Each item reports its element through
 *  `measure(item)`; its last known width counts while it is hidden. `absent` items aren't in this bar;
 *  `dirty` makes room for Save View. */
export function useToolbarFit(bar: RefObject<HTMLElement | null>, absent: readonly Collapsible[] = [], dirty = false) {
  const widths = useRef<Partial<Record<Collapsible, number>>>({});
  const [widthsVersion, setWidthsVersion] = useState(0);
  const [available, setAvailable] = useState(Number.POSITIVE_INFINITY);
  const [hidden, setHidden] = useState<Collapsible[]>([]);

  useLayoutEffect(() => {
    const element = bar.current;
    if (!element || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(() => setAvailable(element.clientWidth - PADDING));
    observer.observe(element);
    return () => observer.disconnect();
  }, [bar]);

  const absentKey = absent.join();
  useLayoutEffect(() => {
    const next = overflowing(available, widths.current, absentKey ? (absentKey.split(",") as Collapsible[]) : [], dirty);
    setHidden((current) => (current.join() === next.join() ? current : next));
  }, [available, widthsVersion, absentKey, dirty]);

  // One ref per item for good, so a render doesn't detach and re-measure every item; a resize re-measures.
  const refs = useRef(new Map<Collapsible, (element: HTMLElement | null) => (() => void) | undefined>());
  const measure = (item: Collapsible) => {
    let ref = refs.current.get(item);
    if (!ref) {
      const record = (element: HTMLElement) => {
        const width = element.getBoundingClientRect().width;
        if (widths.current[item] === width) return;
        widths.current[item] = width;
        setWidthsVersion((v) => v + 1);
      };
      ref = (element) => {
        if (!element) return;
        record(element);
        if (typeof ResizeObserver === "undefined") return;
        const observer = new ResizeObserver(() => record(element));
        observer.observe(element);
        return () => observer.disconnect();
      };
      refs.current.set(item, ref);
    }
    return ref;
  };
  return { hidden, measure };
}
