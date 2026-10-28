import { describe, expect, it } from "vitest";
import { clockTime, timeAgo } from "./relativeTime";

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

describe("clockTime", () => {
  it("formats the local hours and minutes of a UTC timestamp", () => {
    const iso = "2026-03-04T09:05:00Z";

    expect(clockTime(iso)).toBe(new Date(iso).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" }));
    expect(clockTime(iso)).toMatch(/\d{2}\D\d{2}/);
  });
});
