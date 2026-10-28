import { type RefObject, useLayoutEffect, useRef, useState } from "react";
import { type Collapsible, overflowing } from "./fit";

/** The toolbar's horizontal padding (24 each side). */
const PADDING = 48;

/** Watches the toolbar's width and returns the items that don't fit. Each item reports its element through
 *  `measure(item)`; its last known width counts while it is hidden. */
export function useToolbarFit(bar: RefObject<HTMLElement | null>) {
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

  useLayoutEffect(() => {
    const next = overflowing(available, widths.current);
    setHidden((current) => (current.join() === next.join() ? current : next));
  }, [available, widthsVersion]);

  const measure = (item: Collapsible) => (element: HTMLElement | null) => {
    if (!element) return;
    const width = element.getBoundingClientRect().width;
    if (widths.current[item] === width) return;
    widths.current[item] = width;
    setWidthsVersion((v) => v + 1);
  };
  return { hidden, measure };
}
