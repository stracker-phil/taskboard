import { useEffect } from "react";
import { api } from "../api";
import { useSettings } from "../queries/settings";

/** Until the settings load: Rust's default. */
const DEFAULT_SECONDS = 2;

/** Marks the shown issue seen once it has been open for the owner's delay (Settings → General); leaving it
 *  sooner doesn't. */
export function useSeenAfter(id: string, hasUnseen: boolean) {
  const { data: settings } = useSettings();
  const delay = (settings?.behaviour.seenAfterSeconds ?? DEFAULT_SECONDS) * 1000;
  useEffect(() => {
    if (!hasUnseen) return;
    const timer = setTimeout(() => void api.issuesMarkSeen([id]).catch(() => {}), delay);
    return () => clearTimeout(timer);
  }, [id, hasUnseen, delay]);
}
