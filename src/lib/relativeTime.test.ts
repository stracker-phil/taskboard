import { describe, expect, it } from "vitest";
import { clockTime, fullDateTime, shortWhen, timeAgo } from "./relativeTime";

const NOW = new Date("2026-03-04T12:00:00Z");
const before = (ms: number) => new Date(NOW.getTime() - ms).toISOString();
const MIN = 60_000;
const HOUR = 60 * MIN;

describe("timeAgo", () => {
  it.each<[string, number, string]>([
    ["under a minute", 59_000, "just now"],
    ["a future timestamp", -5 * MIN, "just now"],
    ["one minute", MIN, "1 min ago"],
    ["59 minutes", 59 * MIN + 59_000, "59 min ago"],
    ["one hour", HOUR, "1 h ago"],
    ["23 hours", 23 * HOUR + 59 * MIN, "23 h ago"],
  ])("says %s as %s", (_name, ms, text) => {
    expect(timeAgo(before(ms), NOW)).toBe(text);
  });

  it("falls back to the local date after a day", () => {
    const iso = before(30 * HOUR);

    expect(timeAgo(iso, NOW)).toBe(new Date(iso).toLocaleDateString(undefined, { day: "numeric", month: "short" }));
  });
});

describe("shortWhen", () => {
  const LOCAL_NOON = new Date(2026, 2, 4, 12, 0);
  const iso = (date: Date) => date.toISOString();

  it.each<[string, Date, string]>([
    ["under a minute ago", new Date(2026, 2, 4, 11, 59, 30), "just now"],
    ["a future timestamp", new Date(2026, 2, 4, 12, 10), "just now"],
    ["five minutes ago", new Date(2026, 2, 4, 11, 55), "5 min ago"],
    ["59 minutes ago", new Date(2026, 2, 4, 11, 0, 30), "59 min ago"],
  ])("says %s as %s", (_name, at, text) => {
    expect(shortWhen(iso(at), LOCAL_NOON)).toBe(text);
  });

  it("names the clock time for an earlier moment of the same day", () => {
    const at = iso(new Date(2026, 2, 4, 8, 30));

    expect(shortWhen(at, LOCAL_NOON)).toBe(`at ${clockTime(at)}`);
  });

  it("keeps counting minutes across midnight while under an hour", () => {
    const now = new Date(2026, 2, 5, 0, 30);

    expect(shortWhen(iso(new Date(2026, 2, 4, 23, 50)), now)).toBe("40 min ago");
  });

  it("names yesterday and the clock time for a moment on the previous day", () => {
    const at = iso(new Date(2026, 2, 3, 9, 12));

    expect(shortWhen(at, LOCAL_NOON)).toBe(`yesterday at ${clockTime(at)}`);
  });

  it("names yesterday across a month boundary", () => {
    const at = iso(new Date(2026, 1, 28, 9, 12));

    expect(shortWhen(at, new Date(2026, 2, 1, 12, 0))).toBe(`yesterday at ${clockTime(at)}`);
  });

  it("names the day and month and the clock time for anything older", () => {
    const date = new Date(2026, 1, 20, 9, 12);
    const at = iso(date);

    expect(shortWhen(at, LOCAL_NOON)).toBe(`on ${date.toLocaleDateString(undefined, { day: "numeric", month: "short" })} at ${clockTime(at)}`);
  });
});

describe("fullDateTime", () => {
  it("spells out the local date and time down to the seconds", () => {
    const text = fullDateTime(new Date(2026, 2, 4, 14, 2, 10).toISOString());

    expect(text).toContain("2026");
    expect(text).toMatch(/14\D02\D10|2\D02\D10/);
  });
});

describe("clockTime", () => {
  it("formats the local hours and minutes of a UTC timestamp", () => {
    const iso = "2026-03-04T09:05:00Z";

    expect(clockTime(iso)).toBe(new Date(iso).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" }));
    expect(clockTime(iso)).toMatch(/\d{2}\D\d{2}/);
  });
});
