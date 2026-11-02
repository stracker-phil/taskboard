import { describe, expect, it } from "vitest";
import type { MeMatch, MeRule } from "../../api";
import { onOf, rulesFor, sameOn } from "./meRules";

const REPORTER: MeMatch = { kind: "field", field: "reporter" };
const CREATOR: MeMatch = { kind: "field", field: "creator" };
const MENTIONED: MeMatch = { kind: "mentioned" };
const SUBTASKS: MeMatch = { kind: "subtasks" };

const APP: MeRule[] = [
  { rule: REPORTER, on: true },
  { rule: CREATOR, on: false },
  { rule: MENTIONED, on: false },
  { rule: SUBTASKS, on: true },
];

describe("rulesFor", () => {
  it("returns the app's rules when the view follows them", () => {
    expect(rulesFor(APP, null)).toEqual(APP);
  });

  it("switches on exactly the listed rules and everything else off", () => {
    const rules = rulesFor(APP, [MENTIONED, CREATOR]);

    expect(rules.map((r) => r.on)).toEqual([false, true, true, false]);
    expect(rules.map((r) => r.rule)).toEqual(APP.map((r) => r.rule));
  });

  it("matches field rules by field name", () => {
    expect(onOf(rulesFor(APP, [{ kind: "field", field: "creator" }]))).toEqual([CREATOR]);
  });

  it("switches everything off for an empty list", () => {
    expect(onOf(rulesFor(APP, []))).toEqual([]);
  });
});

describe("onOf", () => {
  it("lists the rules that are on, in the app's order", () => {
    expect(onOf(APP)).toEqual([REPORTER, SUBTASKS]);
  });
});

describe("sameOn", () => {
  it("is true for the same rules in another order", () => {
    expect(sameOn(APP, [SUBTASKS, REPORTER], [REPORTER, SUBTASKS])).toBe(true);
  });

  it("is true when one follows the app and the other lists the app's rules that are on", () => {
    expect(sameOn(APP, null, [REPORTER, SUBTASKS])).toBe(true);
  });

  it("is false when one more rule is on", () => {
    expect(sameOn(APP, [REPORTER], [REPORTER, MENTIONED])).toBe(false);
  });

  it("is false when a listed rule is not in the app's rules and the other lists none", () => {
    expect(sameOn(APP, [], [REPORTER])).toBe(false);
  });
});
