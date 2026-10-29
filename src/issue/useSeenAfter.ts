import { useEffect } from "react";
import { api } from "../api";
import { useSettings } from "../queries/settings";

/** Until the settings load: Rust's default. */
const DEFAULT_SECONDS = 2;

let heldUnread: string | null = null;

/** The owner marked the shown issue unread: it stays unread while it is on screen. */
export function holdUnread(id: string) {
  heldUnread = id;
}

/** Marks the shown issue read once it has been open for the owner's delay (Settings → General); leaving it
 *  sooner doesn't, and neither does an issue the owner just marked unread. */
export function useSeenAfter(id: string, unread: boolean) {
  const { data: settings } = useSettings();
  const delay = (settings?.behaviour.seenAfterSeconds ?? DEFAULT_SECONDS) * 1000;
  useEffect(
    () => () => {
      if (heldUnread === id) heldUnread = null;
    },
    [id],
  );
  useEffect(() => {
    if (!unread || heldUnread === id) return;
    const timer = setTimeout(() => void api.issuesMarkSeen([id]).catch(() => {}), delay);
    return () => clearTimeout(timer);
  }, [id, unread, delay]);
}
