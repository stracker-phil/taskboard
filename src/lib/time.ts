const SHORT = new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" });
const TIME = new Intl.DateTimeFormat(undefined, { timeStyle: "short" });

/** A UTC timestamp from Rust as local time: only the time when it is today ("9:30 AM"), else the date too
 *  ("Mar 4, 2026, 9:30 AM"). */
export function formatLocalTime(iso: string, now: Date = new Date()): string {
  const at = new Date(iso);
  return at.toDateString() === now.toDateString() ? TIME.format(at) : SHORT.format(at);
}

const DAY = new Intl.DateTimeFormat(undefined, { day: "numeric", month: "short" });

/** A UTC timestamp from Rust as its local day ("27 Oct"). */
export function formatLocalDate(iso: string): string {
  return DAY.format(new Date(iso));
}
