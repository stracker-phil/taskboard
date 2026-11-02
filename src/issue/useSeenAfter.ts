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
  const delay = useSeenDelay();
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

/** Counts the previewed issue as used once it has been shown for the same delay, so passing it doesn't. */
export function usePreviewedAfter(id: string) {
  const delay = useSeenDelay();
  useEffect(() => {
    const timer = setTimeout(() => void api.issuePreviewed(id).catch(() => {}), delay);
    return () => clearTimeout(timer);
  }, [id, delay]);
}

function useSeenDelay(): number {
  const { data: settings } = useSettings();
  return (settings?.behaviour.seenAfterSeconds ?? DEFAULT_SECONDS) * 1000;
}
