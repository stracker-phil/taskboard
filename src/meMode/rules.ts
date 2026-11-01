import type { MeMatch, UserField } from "../api";

/** How a Me rule reads in the Me popup and in Settings; a field the cache no longer knows goes by its id. */
export function ruleLabel(rule: MeMatch, fields: UserField[]): string {
  switch (rule.kind) {
    case "field":
      return fields.find((f) => f.field === rule.field)?.name ?? rule.field;
    case "mentioned":
      return "Mentioned in a comment";
    case "subtasks":
      return "Subtasks of my issues";
  }
}

export function ruleKey(rule: MeMatch): string {
  return rule.kind === "field" ? `field:${rule.field}` : rule.kind;
}
