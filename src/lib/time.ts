import type { LocalDays } from "../api";

export type Day = "today" | "yesterday" | "earlier";

/** Which of the owner's days a UTC timestamp falls on, by the day boundaries Rust reports (`local_days`, ADR 0074);
 *  "earlier" while they haven't loaded. */
export function dayOf(iso: string, days: LocalDays | undefined): Day {
  if (!days) return "earlier";
  const at = Date.parse(iso);
  if (at >= Date.parse(days.today) && at < Date.parse(days.tomorrow)) return "today";
  if (at >= Date.parse(days.yesterday) && at < Date.parse(days.today)) return "yesterday";
  return "earlier";
}

const formatters = new Map<string, Intl.DateTimeFormat>();

/** The platform's formatter for these options in the owner's zone (the Mac's while the days haven't loaded). */
export function formatter(options: Intl.DateTimeFormatOptions, days?: LocalDays): Intl.DateTimeFormat {
  const timeZone = days?.timeZone;
  const key = JSON.stringify([options, timeZone]);
  let found = formatters.get(key);
  if (!found) {
    found = new Intl.DateTimeFormat(undefined, { ...options, timeZone });
    formatters.set(key, found);
  }
  return found;
}

/** A UTC timestamp from Rust as local time: only the time when it is today ("9:30 AM"), else the date too
 *  ("Mar 4, 2026, 9:30 AM"). */
export function formatLocalTime(iso: string, days: LocalDays | undefined): string {
  const today = dayOf(iso, days) === "today";
  return formatter(today ? { timeStyle: "short" } : { dateStyle: "medium", timeStyle: "short" }, days).format(new Date(iso));
}

/** A UTC timestamp from Rust as its day in the owner's zone ("27 Oct"). */
export function formatLocalDate(iso: string, days?: LocalDays): string {
  return formatter({ day: "numeric", month: "short" }, days).format(new Date(iso));
}
