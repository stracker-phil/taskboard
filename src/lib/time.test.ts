import { describe, expect, it } from "vitest";
import { formatLocalDate, formatLocalTime } from "./time";

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
  const now = new Date(2026, 9, 28, 18, 0);

  it("shows the date with the year when the timestamp is from another day", () => {
    const yesterday = new Date(2026, 9, 27, 9, 30).toISOString();

    expect(formatLocalTime(yesterday, now)).toContain("2026");
  });

  it("shows only the time when the timestamp is from the same local day", () => {
    const earlierToday = new Date(2026, 9, 28, 9, 30).toISOString();
    const text = formatLocalTime(earlierToday, now);

    expect(text).not.toBe("");
    expect(text).not.toContain("2026");
  });

  it("treats a timestamp just after local midnight as the same day", () => {
    const justAfterMidnight = new Date(2026, 9, 28, 0, 5).toISOString();

    expect(formatLocalTime(justAfterMidnight, now)).not.toContain("2026");
  });
});
