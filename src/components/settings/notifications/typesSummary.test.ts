import { describe, expect, it } from "vitest";
import { typesSummary } from "./typesSummary";

const TYPES = ["Bug", "Story", "Task", "Test", "Test Execution"];

describe("typesSummary", () => {
  it.each<[string, string[], string]>([
    ["nothing is left out", [], "All types"],
    ["every type is left out", TYPES, "No types"],
    ["fewer types are left out than kept", ["Test", "Test Execution"], "All except Test, Test Execution"],
    ["fewer types are kept than left out", ["Task", "Test", "Test Execution"], "Only Bug, Story"],
  ])("reads for %s", (_case, excluded, label) => {
    expect(typesSummary(TYPES, excluded)).toBe(label);
  });

  it("counts the types left out when the names would pass 40 characters", () => {
    const types = ["Infrastructure Request", "Customer Escalation", "Bug", "Story", "Task"];

    expect(typesSummary(types, ["Infrastructure Request", "Customer Escalation"])).toBe("2 types left out");
  });

  it("uses the singular for one type left out of a long list", () => {
    const types = ["Bug", "Infrastructure Request Escalation Review", "Story"];

    expect(typesSummary(types, ["Infrastructure Request Escalation Review"])).toBe("1 type left out");
  });

  it("ignores excluded names that are no longer a type", () => {
    expect(typesSummary(["Bug", "Story"], ["Retired"])).toBe("All types");
  });
});
