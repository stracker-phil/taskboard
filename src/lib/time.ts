const SHORT = new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" });

/** A UTC timestamp from Rust as a short local date and time, e.g. "Mar 4, 2026, 9:30 AM". */
export const formatLocalTime = (iso: string) => SHORT.format(new Date(iso));
