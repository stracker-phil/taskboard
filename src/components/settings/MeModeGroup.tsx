import { Plus } from "lucide-react";
import { api, type MeRuleEdit, type Settings } from "../../api";
import { ruleKey, ruleLabel } from "../../meMode/rules";
import { useMeUserFields } from "../../queries/views";
import { attempt } from "../../state/requests";
import { openMenu } from "../menu";
import { Group, Row, SwitchInput } from "./controls";

export function MeModeGroup({ settings }: { settings: Settings }) {
  const { data: fields = [] } = useMeUserFields();
  const rules = settings.behaviour.meRules;
  const save = (edit: MeRuleEdit) => void attempt(api.meRuleEdit(edit), "Could not save Me mode");
  const offered = new Set(rules.map((r) => ruleKey(r.rule)));
  const addable = fields.filter((f) => !offered.has(ruleKey({ kind: "field", field: f.field })));
  const add = (button: HTMLElement) =>
    openMenu(
      addable.length > 0 ? addable.map((f) => ({ text: f.name, run: () => save({ kind: "addField", field: f.field }) })) : [{ text: "No other user fields", enabled: false }],
      button,
      "Add a field to Me mode",
    );

  return (
    <Group
      title="Me Mode"
      anchor="meMode"
      description="Me mode always shows the issues assigned to you. Switch on what else counts as yours; the Me popup in the toolbar has the same switches."
    >
      {rules.map(({ rule, on }) => {
        const label = ruleLabel(rule, fields);
        return (
          <Row key={ruleKey(rule)} label={label}>
            {rule.kind === "field" && (
              <button type="button" className="btn" aria-label={`Remove ${label} from Me mode`} onClick={() => save({ kind: "removeField", field: rule.field })}>
                Remove
              </button>
            )}
            <SwitchInput aria-label={`Count ${label}`} checked={on} onChange={(next) => save({ kind: "switch", rule, on: next })} />
          </Row>
        );
      })}
      <Row label="Fields" description="A user field such as a project's own Developer field: issues where it names you count as yours.">
        <button type="button" className="btn" aria-haspopup="menu" onClick={(e) => add(e.currentTarget)}>
          <Plus size={12} aria-hidden="true" />
          Add Field…
        </button>
      </Row>
    </Group>
  );
}
