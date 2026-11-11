import type { CapabilityRow, TokenKind } from "../../../api";
import { useCapabilities, useSettings } from "../../../queries/settings";
import { Group, Row } from "../controls";

interface ScopeNeed {
  scope: string;
  required: boolean;
  /** At least one change using it is switched on while changes are allowed. */
  inUse: boolean;
  labels: string[];
}

function needsOf(rows: CapabilityRow[], allowChanges: boolean, disabled: string[]): ScopeNeed[] {
  const needs = new Map<string, ScopeNeed>();
  for (const row of rows) {
    const need = needs.get(row.scope) ?? { scope: row.scope, required: false, inUse: false, labels: [] };
    need.labels.push(row.label);
    if (!row.change) need.required = true;
    else if (allowChanges && !disabled.includes(row.capability)) need.inUse = true;
    needs.set(row.scope, need);
  }
  return [...needs.values()];
}

function needText(need: ScopeNeed): string {
  if (need.required) return "Required";
  return need.inUse ? "Needed for changes" : "Optional";
}

/** Every scope a scoped token needs, in one list, each with what Taskboard uses it for. */
export function TokenScopesGroup({ tokenKind }: { tokenKind: TokenKind }) {
  const { data: settings } = useSettings();
  const { data: capabilities } = useCapabilities();
  if (!settings || !capabilities) return null;
  const { allowJiraChanges, disabledChanges } = settings.behaviour;
  const needs = needsOf(capabilities, allowJiraChanges, disabledChanges);
  return (
    <Group
      title="Token scopes"
      anchor="scopes"
      description={
        tokenKind === "scoped"
          ? "Give your scoped token these scopes."
          : "Only for a scoped token; a classic token acts with all of your account's access."
      }
    >
      {needs.map((need) => (
        <Row
          key={need.scope}
          label={<code className="settings-scope">{need.scope}</code>}
          description={`Used for: ${need.labels.join(", ")}.`}
        >
          <span className="settings-value">{needText(need)}</span>
        </Row>
      ))}
    </Group>
  );
}
