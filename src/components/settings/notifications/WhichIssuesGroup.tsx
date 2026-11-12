import { useId, type ReactNode } from "react";
import { api, type Involvement, type Notifications, type NotifyScope } from "../../../api";
import { useNotificationIssueTypes, usePeopleFields } from "../../../queries/settings";
import { ValuePicker } from "../../../shell/toolbar/ValuePicker";
import { attempt } from "../../../state/requests";
import { Group, Row } from "../controls";
import { typesSummary } from "./typesSummary";
import { useSaveNotifications } from "./useSaveNotifications";

const SCOPES: { scope: NotifyScope; label: string }[] = [
  { scope: "allIssues", label: "All issues in my Spaces" },
  { scope: "involved", label: "Only issues I'm involved in" },
];

const INVOLVEMENT: { key: keyof Involvement; label: string }[] = [
  { key: "assigned", label: "Assigned to me" },
  { key: "reported", label: "Created by me" },
  { key: "peopleFields", label: "Named in a people field" },
];

function TickRow({ type, label, description, checked, disabled, onChange, name }: {
  type: "radio" | "checkbox";
  label: string;
  description?: ReactNode;
  checked: boolean;
  disabled?: boolean;
  onChange: (on: boolean) => void;
  name?: string;
}) {
  const id = useId();
  return (
    <Row label={label} description={description} htmlFor={id} disabled={disabled}>
      <input id={id} className="settings-check" type={type} name={name} checked={checked} disabled={disabled} onChange={(e) => onChange(e.target.checked)} />
    </Row>
  );
}

function PeopleFieldsHint() {
  const { data: fields } = usePeopleFields();
  if (!fields) return null;
  if (fields.length > 0) return <>{fields.map((field) => field.name).join(", ")}</>;
  return (
    <>
      Opt in a user field under{" "}
      <button type="button" className="settings-link" onClick={() => void attempt(api.runCommand("app:settings:fields"), "Could not open the Custom fields settings")}>
        Custom fields
      </button>{" "}
      to use it.
    </>
  );
}

/** Notifications → Which issues: the filters both channels share; the changes about you ignore them. */
export function WhichIssuesGroup({ notifications }: { notifications: Notifications }) {
  const save = useSaveNotifications();
  const scopeName = useId();
  const typesId = useId();
  const { data: types = [] } = useNotificationIssueTypes();
  const { scope, involvement, excludedTypes, ignoreAutomations } = notifications;
  // The involvement rows stay in place and only dim, so picking a scope moves nothing.
  const allIssues = scope === "allIssues";

  return (
    <Group title="Which issues" anchor="which-issues" description="The issues whose changes count, in app and on macOS.">
      {SCOPES.map((option) => (
        <TickRow key={option.scope} type="radio" name={scopeName} label={option.label} checked={scope === option.scope} onChange={() => save({ scope: option.scope })} />
      ))}
      {INVOLVEMENT.map(({ key, label }) => (
        <TickRow
          key={key}
          type="checkbox"
          label={label}
          description={key === "peopleFields" ? <PeopleFieldsHint /> : undefined}
          checked={involvement[key]}
          disabled={allIssues}
          onChange={(on) => save({ involvement: { ...involvement, [key]: on } })}
        />
      ))}
      <Row label="Issue types" htmlFor={typesId} description="A type left out counts only for the changes about you.">
        <ValuePicker
          id={typesId}
          choices={types.map((name) => ({ id: name, name }))}
          selected={types.filter((type) => !excludedTypes.includes(type))}
          summary={typesSummary(types, excludedTypes)}
          onChange={(kept) => save({ excludedTypes: types.filter((type) => !kept.includes(type)) })}
        />
      </Row>
      <TickRow
        type="checkbox"
        label="Ignore changes made by automations"
        description="Changes and comments by apps (Xray, CI, Jira Automation) stay in the activity but never make an issue unread or notify."
        checked={ignoreAutomations}
        onChange={(on) => save({ ignoreAutomations: on })}
      />
    </Group>
  );
}
