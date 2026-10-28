import { describe, expect, it } from "vitest";
import type { Condition, FilterField, FilterOptions, Op, ValueKind } from "../../api";
import { choicesFor, isComplete, newCondition, opsFor, pickedValues, shownOp, startValue, takesDays, takesNoValue, withValues } from "./conditionEditing";

const OPTIONS: FilterOptions = {
  fields: [],
  statuses: [
    { id: "3", name: "In Progress" },
    { id: "4", name: "Done" },
  ],
  types: [{ id: "Task", name: "Task" }],
  users: [{ id: "demo-account-2", name: "Sam Lee" }],
  priorities: [{ id: "High", name: "High" }],
  epics: [],
  releases: [],
  sprints: [],
  labels: [],
  projects: [{ id: "NW", name: "Website relaunch" }],
};

const field = (id: string, kind: ValueKind): FilterField => ({ id, name: id, kind });

describe("opsFor", () => {
  it.each<[ValueKind, string[]]>([
    ["status", ["is", "isNot", "isEmpty", "isNotEmpty"]],
    ["statusCategory", ["is", "isNot"]],
    ["project", ["is", "isNot"]],
    ["date", ["before", "after", "withinLastDays", "withinNextDays", "isEmpty", "isNotEmpty"]],
    ["text", ["contains", "is", "isEmpty", "isNotEmpty"]],
    ["search", ["contains"]],
    ["number", ["is", "isNot", "isEmpty", "isNotEmpty"]],
  ])("offers the operators of a %s field", (kind, ops) => {
    expect(opsFor(kind)).toEqual(ops);
  });
});

describe("operator kinds", () => {
  it("knows which operators take no value", () => {
    expect([takesNoValue("isEmpty"), takesNoValue("isNotEmpty"), takesNoValue("is")]).toEqual([true, true, false]);
  });

  it("knows which operators count days", () => {
    expect([takesDays("withinLastDays"), takesDays("withinNextDays"), takesDays("before")]).toEqual([true, true, false]);
  });
});

describe("choicesFor", () => {
  it("offers the cached values of a picker field", () => {
    expect(choicesFor("status", OPTIONS)).toBe(OPTIONS.statuses);
  });

  it("puts Me before the people of a user field", () => {
    expect(choicesFor("user", OPTIONS)).toEqual([{ id: "me", name: "Me" }, { id: "demo-account-2", name: "Sam Lee" }]);
  });

  it("offers the three status categories", () => {
    expect(choicesFor("statusCategory", OPTIONS)?.map((c) => c.id)).toEqual(["new", "indeterminate", "done"]);
  });

  it.each<ValueKind>(["text", "date", "number"])("offers no choices for a typed %s field", (kind) => {
    expect(choicesFor(kind, OPTIONS)).toBeNull();
  });
});

describe("startValue", () => {
  it("is nothing for an operator without a value", () => {
    expect(startValue(field("status", "status"), "isEmpty", OPTIONS)).toBeUndefined();
  });

  it("is 7 days for a day-counting operator", () => {
    expect(startValue(field("updated", "date"), "withinLastDays", OPTIONS)).toBe(7);
  });

  it("is the first choice of a picker", () => {
    expect(startValue(field("status", "status"), "is", OPTIONS)).toBe("3");
  });

  it("is Me for a user field", () => {
    expect(startValue(field("assignee", "user"), "is", OPTIONS)).toBe("me");
  });

  it("is empty text for a picker without cached choices", () => {
    expect(startValue(field("epic", "epic"), "is", OPTIONS)).toBe("");
  });

  it("is empty text for a typed field", () => {
    expect(startValue(field("customfield_10071", "text"), "contains", OPTIONS)).toBe("");
  });
});

describe("newCondition", () => {
  it("uses the field's first operator and its start value", () => {
    expect(newCondition(field("priority", "priority"), OPTIONS)).toEqual({ field: "priority", op: "is", value: "High" });
  });
});

describe("pickedValues", () => {
  it.each<[string, Condition, string[]]>([
    ["a single string", { field: "status", op: "is", value: "3" }, ["3"]],
    ["a list", { field: "status", op: "in", value: ["3", "4"] }, ["3", "4"]],
    ["an empty string", { field: "status", op: "is", value: "" }, []],
    ["a missing value", { field: "status", op: "is" }, []],
    ["a number", { field: "updated", op: "withinLastDays", value: 7 }, []],
  ])("lists the values of %s", (_, condition, expected) => {
    expect(pickedValues(condition)).toEqual(expected);
  });
});

describe("shownOp", () => {
  it.each<[Op, Op]>([
    ["in", "is"],
    ["notIn", "isNot"],
    ["is", "is"],
    ["isNot", "isNot"],
    ["contains", "contains"],
  ])("shows %s as %s", (op, shown) => {
    expect(shownOp(op)).toBe(shown);
  });
});

describe("withValues", () => {
  it.each<[string, Op, string[], Condition]>([
    ["one value as is", "is", ["3"], { field: "status", op: "is", value: "3" }],
    ["one value as is not", "isNot", ["3"], { field: "status", op: "isNot", value: "3" }],
    ["several values as in", "is", ["3", "4"], { field: "status", op: "in", value: ["3", "4"] }],
    ["several values as not in", "isNot", ["3", "4"], { field: "status", op: "notIn", value: ["3", "4"] }],
    ["no value as is", "is", [], { field: "status", op: "is", value: "" }],
    ["no value as is not", "isNot", [], { field: "status", op: "isNot", value: "" }],
    ["one value back from in", "in", ["3"], { field: "status", op: "is", value: "3" }],
    ["one value back from not in", "notIn", ["3"], { field: "status", op: "isNot", value: "3" }],
    ["several values staying not in", "notIn", ["3", "4"], { field: "status", op: "notIn", value: ["3", "4"] }],
  ])("sets %s", (_, op, values, expected) => {
    expect(withValues({ field: "status", op, value: "x" }, values)).toEqual(expected);
  });
});

describe("isComplete", () => {
  it.each<[string, Condition, boolean]>([
    ["an operator without a value", { field: "due", op: "isEmpty" }, true],
    ["a picked value", { field: "status", op: "is", value: "3" }, true],
    ["empty text", { field: "customfield_10071", op: "contains", value: "" }, false],
    ["blank text", { field: "customfield_10071", op: "contains", value: "   " }, false],
    ["a missing value", { field: "status", op: "is" }, false],
    ["a positive day count", { field: "updated", op: "withinLastDays", value: 7 }, true],
    ["a zero day count", { field: "updated", op: "withinLastDays", value: 0 }, false],
    ["a list of values", { field: "labels", op: "in", value: ["web"] }, true],
    ["an empty list of values", { field: "labels", op: "in", value: [] }, false],
  ])("judges %s", (_, condition, expected) => {
    expect(isComplete(condition)).toBe(expected);
  });
});
