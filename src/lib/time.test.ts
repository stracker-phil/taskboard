import { describe, expect, it } from "vitest";
import { BERLIN_DAYS, localDays } from "../test/days";
import { dayOf, formatLocalDate, formatLocalTime } from "./time";

describe("dayOf", () => {
  it.each<[string, string, string]>([
    ["the first instant of today", BERLIN_DAYS.today, "today"],
    ["the last millisecond of today", "2026-11-12T22:59:59.999Z", "today"],
    ["the first instant of tomorrow", BERLIN_DAYS.tomorrow, "earlier"],
    ["the last millisecond of yesterday", "2026-11-11T22:59:59.999Z", "yesterday"],
    ["the first instant of yesterday", BERLIN_DAYS.yesterday, "yesterday"],
    ["the last millisecond before yesterday", "2026-11-10T22:59:59.999Z", "earlier"],
    ["a month ago", "2026-10-12T10:00:00Z", "earlier"],
  ])("puts %s on %s", (_name, iso, expected) => {
    expect(dayOf(iso, BERLIN_DAYS)).toBe(expected);
  });

  it("calls everything earlier while the days have not loaded", () => {
    expect(dayOf(BERLIN_DAYS.today, undefined)).toBe("earlier");
  });
});

describe("formatLocalDate", () => {
  it("formats a timestamp as its local day and short month", () => {
    const at = new Date(2026, 9, 27, 12, 0).toISOString();

    const text = formatLocalDate(at);

    expect(text).toContain("27");
    expect(text).toMatch(/Oct/);
    expect(text).not.toContain("2026");
  });
});

describe("formatLocalTime", () => {
  it("shows the date with the year when the timestamp is from another day", () => {
    const yesterday = "2026-11-11T08:30:00Z";

    expect(formatLocalTime(yesterday, BERLIN_DAYS)).toContain("2026");
  });

  it("shows only the time when the timestamp is from today", () => {
    const text = formatLocalTime("2026-11-12T08:30:00Z", BERLIN_DAYS);

    expect(text).toMatch(/9:30/);
    expect(text).not.toContain("2026");
  });

  it("shows the date while the days have not loaded", () => {
    expect(formatLocalTime("2026-11-12T08:30:00Z", undefined)).toContain("2026");
  });

  it("treats a timestamp just after local midnight in Berlin as today", () => {
    const text = formatLocalTime("2026-11-11T23:30:00Z", BERLIN_DAYS);

    expect(text).toMatch(/12:30/);
    expect(text).not.toContain("2026");
  });

  it("treats a timestamp just before local midnight in Berlin as another day, shown in Berlin time", () => {
    const text = formatLocalTime("2026-11-11T22:30:00Z", BERLIN_DAYS);

    expect(text).toContain("2026");
    expect(text).toMatch(/11:30/);
  });

  it("formats in the zone of the days", () => {
    const utc = localDays({ today: "2026-11-12T00:00:00.000Z", yesterday: "2026-11-11T00:00:00.000Z", tomorrow: "2026-11-13T00:00:00.000Z" });

    expect(formatLocalTime("2026-11-12T08:30:00Z", utc)).toMatch(/8:30/);
    expect(formatLocalTime("2026-11-12T08:30:00Z", BERLIN_DAYS)).toMatch(/9:30/);
  });
});
