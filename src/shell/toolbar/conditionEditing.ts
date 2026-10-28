import type { Choice, Condition, FilterField, FilterOptions, Op, Value, ValueKind } from "../../api";

export const OP_NAMES: Record<Op, string> = {
  is: "is",
  isNot: "is not",
  in: "in",
  notIn: "not in",
  isEmpty: "is empty",
  isNotEmpty: "is set",
  contains: "contains",
  before: "before",
  after: "after",
  withinLastDays: "in last",
  withinNextDays: "in next",
};

const PICKED: Op[] = ["is", "isNot", "isEmpty", "isNotEmpty"];
const OPS: Record<ValueKind, Op[]> = {
  status: PICKED,
  statusCategory: ["is", "isNot"],
  type: PICKED,
  user: PICKED,
  priority: PICKED,
  epic: PICKED,
  release: PICKED,
  sprint: PICKED,
  label: PICKED,
  project: ["is", "isNot"],
  date: ["before", "after", "withinLastDays", "withinNextDays", "isEmpty", "isNotEmpty"],
  text: ["contains", "is", "isEmpty", "isNotEmpty"],
  number: ["is", "isNot", "isEmpty", "isNotEmpty"],
};

const STATUS_CATEGORIES: Choice[] = [
  { id: "new", name: "To do" },
  { id: "indeterminate", name: "In progress" },
  { id: "done", name: "Done" },
];

export const opsFor = (kind: ValueKind): Op[] => OPS[kind];

export const takesNoValue = (op: Op) => op === "isEmpty" || op === "isNotEmpty";
export const DEFAULT_DAYS = 7;

export const takesDays = (op: Op) => op === "withinLastDays" || op === "withinNextDays";

/** The choices a picker offers for a field, or null when the value is typed. */
export function choicesFor(kind: ValueKind, options: FilterOptions): Choice[] | null {
  switch (kind) {
    case "status":
      return options.statuses;
    case "statusCategory":
      return STATUS_CATEGORIES;
    case "type":
      return options.types;
    case "user":
      return [{ id: "me", name: "Me" }, ...options.users];
    case "priority":
      return options.priorities;
    case "epic":
      return options.epics;
    case "release":
      return options.releases;
    case "sprint":
      return options.sprints;
    case "label":
      return options.labels;
    case "project":
      return options.projects;
    default:
      return null;
  }
}

/** The value a fresh condition starts with for `field` and `op`. */
export function startValue(field: FilterField, op: Op, options: FilterOptions): Value | undefined {
  if (takesNoValue(op)) return undefined;
  if (takesDays(op)) return DEFAULT_DAYS;
  return choicesFor(field.kind, options)?.[0]?.id ?? "";
}

export function newCondition(field: FilterField, options: FilterOptions): Condition {
  const op = opsFor(field.kind)[0];
  return { field: field.id, op, value: startValue(field, op, options) };
}

/** Only conditions with the value their operator needs are saved. */
export function isComplete(condition: Condition): boolean {
  if (takesNoValue(condition.op)) return true;
  const value = condition.value;
  if (typeof value === "number") return value > 0;
  if (typeof value === "string") return value.trim() !== "";
  return Array.isArray(value) && value.length > 0;
}

/** A picked field's values as a list: one value, several (`in`/`not in`), or none yet. */
export function pickedValues(condition: Condition): string[] {
  const value = condition.value;
  if (Array.isArray(value)) return value;
  return typeof value === "string" && value !== "" ? [value] : [];
}

/** How the operator menu shows it: `in` reads as "is" and `not in` as "is not"; the values say "any of". */
export function shownOp(op: Op): Op {
  if (op === "in") return "is";
  if (op === "notIn") return "isNot";
  return op;
}

/** The condition for these picked values: one keeps `is`/`is not`, several become `in`/`not in`. */
export function withValues(condition: Condition, values: string[]): Condition {
  const negated = shownOp(condition.op) === "isNot";
  if (values.length > 1) return { ...condition, op: negated ? "notIn" : "in", value: values };
  return { ...condition, op: negated ? "isNot" : "is", value: values[0] ?? "" };
}
