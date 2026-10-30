import { type RefObject, type UIEvent, useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { narrowingKey } from "../state/slices/narrowing";
import { useStore } from "../state/store";

const SCROLL_SAVE_MS = 150;

/** Keeps a view's scroll in its place without writing the store on every scroll event: a moment after the
 *  scrolling stops, and when the view unmounts. Scrolls back there once the place has loaded. */
export function useRememberedScroll(listId: number, viewId: number, scrollRef: RefObject<HTMLElement | null>) {
  const key = narrowingKey(listId, viewId);
  const loaded = useStore((s) => key in s.places);
  const pending = useRef<number | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  const flush = useCallback(() => {
    clearTimeout(timer.current);
    if (pending.current === null) return;
    useStore.getState().updatePlace(listId, viewId, { scrollTop: pending.current });
    pending.current = null;
  }, [listId, viewId]);
  useEffect(() => flush, [flush]);

  const onScroll = useCallback(
    (e: UIEvent<HTMLElement>) => {
      pending.current = e.currentTarget.scrollTop;
      clearTimeout(timer.current);
      timer.current = setTimeout(flush, SCROLL_SAVE_MS);
    },
    [flush],
  );

  const [restored, setRestored] = useState(false);
  useLayoutEffect(() => {
    if (!loaded || restored || !scrollRef.current) return;
    scrollRef.current.scrollTop = useStore.getState().places[key]?.scrollTop ?? 0;
    setRestored(true);
  }, [loaded, restored, key, scrollRef]);

  return onScroll;
}
