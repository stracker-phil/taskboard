import { describe, expect, it } from "vitest";
import { BERLIN_DAYS } from "../test/days";
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
  const BERLIN_NOON = new Date("2026-11-12T11:00:00Z");
  const berlin = (iso: string, options: Intl.DateTimeFormatOptions) => new Intl.DateTimeFormat(undefined, { ...options, timeZone: "Europe/Berlin" }).format(new Date(iso));
  const berlinClock = (iso: string) => berlin(iso, { hour: "2-digit", minute: "2-digit" });
  const berlinDate = (iso: string) => berlin(iso, { day: "numeric", month: "short" });

  it.each<[string, string, string]>([
    ["under a minute ago", "2026-11-12T10:59:30Z", "just now"],
    ["a future timestamp", "2026-11-12T11:10:00Z", "just now"],
    ["five minutes ago", "2026-11-12T10:55:00Z", "5 min ago"],
    ["59 minutes ago", "2026-11-12T10:00:30Z", "59 min ago"],
  ])("says %s as %s", (_name, at, text) => {
    expect(shortWhen(at, BERLIN_NOON, BERLIN_DAYS)).toBe(text);
  });

  it("names the clock time in the owner's zone for an earlier moment of today", () => {
    const at = "2026-11-12T07:30:00Z";

    expect(shortWhen(at, BERLIN_NOON, BERLIN_DAYS)).toBe(`at ${berlinClock(at)}`);
  });

  it("keeps counting minutes across midnight while under an hour", () => {
    const now = new Date("2026-11-11T23:30:00Z");

    expect(shortWhen("2026-11-11T22:50:00Z", now, BERLIN_DAYS)).toBe("40 min ago");
  });

  it("calls a moment just after local midnight today, though it is still the day before in UTC", () => {
    const at = "2026-11-11T23:00:00Z";

    expect(shortWhen(at, BERLIN_NOON, BERLIN_DAYS)).toBe(`at ${berlinClock(at)}`);
  });

  it("calls a moment just before local midnight yesterday", () => {
    const at = "2026-11-11T22:59:00Z";

    expect(shortWhen(at, BERLIN_NOON, BERLIN_DAYS)).toBe(`yesterday at ${berlinClock(at)}`);
  });

  it("names the day and month in the owner's zone and the clock time for anything older", () => {
    const at = "2026-10-20T22:30:00Z";

    expect(shortWhen(at, BERLIN_NOON, BERLIN_DAYS)).toBe(`on ${berlinDate(at)} at ${berlinClock(at)}`);
    expect(berlinDate(at)).toContain("21");
  });

  it("names the day and month for any moment while the days have not loaded", () => {
    const at = "2026-11-12T07:30:00Z";

    expect(shortWhen(at, BERLIN_NOON, undefined)).toMatch(/^on .+ at /);
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
