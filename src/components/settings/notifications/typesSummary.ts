/** Past this many characters the label only counts the types left out. */
const MAX_LABEL = 40;

/** The type picker's label: "All types", "All except Test, Test Execution", "Only Bug, Story" or "3 types left out". */
export function typesSummary(types: string[], excluded: string[]): string {
  const left = types.filter((type) => excluded.includes(type));
  if (left.length === 0) return "All types";
  const kept = types.filter((type) => !excluded.includes(type));
  if (kept.length === 0) return "No types";
  const label = kept.length < left.length ? `Only ${kept.join(", ")}` : `All except ${left.join(", ")}`;
  return label.length > MAX_LABEL ? `${left.length} ${left.length === 1 ? "type" : "types"} left out` : label;
}
