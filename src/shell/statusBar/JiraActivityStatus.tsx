import { useEffect, useState } from "react";
import { useJiraActivity } from "../../queries/sync";

/** Requests shorter than this never show; a pause this short between two requests doesn't blink. */
const SHOW_AFTER_MS = 300;
const HIDE_AFTER_MS = 250;

/** The newest request in words with how many others run, e.g. "Saving the status… +1". */
export function activityText(doing: string[]): string {
  const newest = doing[doing.length - 1];
  return doing.length > 1 ? `${newest}… +${doing.length - 1}` : `${newest}…`;
}

/** Every request in flight, alike ones counted: "Fetching issues ×3". */
export function activityDetail(doing: string[]): string {
  const counts = new Map<string, number>();
  for (const entry of doing) counts.set(entry, (counts.get(entry) ?? 0) + 1);
  return [...counts].map(([entry, count]) => (count > 1 ? `${entry} ×${count}` : entry)).join("\n");
}

/** A spinner and what Taskboard is asking Jira, while a request runs longer than a moment. */
export function JiraActivityStatus() {
  const doing = useJiraActivity();
  const shown = useSettled(doing);
  if (shown.length === 0) return null;
  return (
    <span className="status-activity" role="status" title={activityDetail(shown)}>
      <span className="status-activity__spinner" aria-hidden="true" />
      <span className="status-activity__text">{activityText(shown)}</span>
    </span>
  );
}

/** `doing` once it has been non-empty for `SHOW_AFTER_MS`, and until it has been empty for `HIDE_AFTER_MS`. */
function useSettled(doing: string[]): string[] {
  const [shown, setShown] = useState<string[]>([]);
  const busy = doing.length > 0;
  const showing = shown.length > 0;
  useEffect(() => {
    if (busy === showing) {
      if (busy) setShown(doing);
      return;
    }
    const timer = setTimeout(() => setShown(doing), busy ? SHOW_AFTER_MS : HIDE_AFTER_MS);
    return () => clearTimeout(timer);
  }, [doing, busy, showing]);
  return shown;
}
