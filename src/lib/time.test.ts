import { describe, expect, it } from "vitest";
import { formatLocalTime } from "./time";

describe("formatLocalTime", () => {
  it("formats an ISO timestamp as a non-empty local date and time", () => {
    const text = formatLocalTime("2026-03-04T09:30:00Z");

    expect(text).not.toBe("");
    expect(text).toContain("2026");
  });
});
