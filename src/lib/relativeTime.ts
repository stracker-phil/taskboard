/** "just now", "2 min ago", "3 h ago", then the local time: how long ago a UTC timestamp was. */
export function timeAgo(iso: string, now: Date = new Date()): string {
  const minutes = Math.floor((now.getTime() - new Date(iso).getTime()) / 60_000);
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} h ago`;
  return new Date(iso).toLocaleDateString(undefined, { day: "numeric", month: "short" });
}

/** The local clock time of a UTC timestamp ("14:02"). */
export function clockTime(iso: string): string {
  return new Date(iso).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
}

const FULL = new Intl.DateTimeFormat(undefined, { dateStyle: "full", timeStyle: "medium" });
const DAY_MONTH = new Intl.DateTimeFormat(undefined, { day: "numeric", month: "short" });

/** When something happened, short enough for the status bar: "just now", "5 min ago", "at 14:02",
 *  "yesterday at 09:12", "on 3 Nov at 09:12". */
export function shortWhen(iso: string, now: Date = new Date()): string {
  const at = new Date(iso);
  const minutes = Math.floor((now.getTime() - at.getTime()) / 60_000);
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes} min ago`;
  const yesterday = new Date(now);
  yesterday.setDate(now.getDate() - 1);
  if (at.toDateString() === now.toDateString()) return `at ${clockTime(iso)}`;
  if (at.toDateString() === yesterday.toDateString()) return `yesterday at ${clockTime(iso)}`;
  return `on ${DAY_MONTH.format(at)} at ${clockTime(iso)}`;
}

/** The full local date and time of a UTC timestamp ("Wednesday, 11 November 2026 at 14:02:10"). */
export function fullDateTime(iso: string): string {
  return FULL.format(new Date(iso));
}
