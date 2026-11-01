import { api, type CapabilityRow } from "../../../api";
import { useCapabilities, useSettings } from "../../../queries/settings";
import { attempt } from "../../../state/requests";
import { Group, Row, Switch, SwitchInput } from "../controls";

/** Changes to Jira on or off, then each kind of change on its own, then what Taskboard reads, each with the token
 *  scope it needs. The list comes from the gateway's own requests (`jira::capability`), so it is what the app does. */
export function ChangesGroup() {
  const { data: settings } = useSettings();
  const { data: capabilities = [] } = useCapabilities();
  if (!settings) return null;
  const { behaviour } = settings;
  const setAllowJiraChanges = (allowJiraChanges: boolean) =>
    void attempt(api.behaviourSet({ ...behaviour, allowJiraChanges }), "Could not save the setting");
  const setChange = (row: CapabilityRow, enabled: boolean) =>
    void attempt(api.jiraCapabilitySet(row.capability, enabled), `Could not change “${row.label}”`);
  const changes = capabilities.filter((row) => row.change);
  const reads = capabilities.filter((row) => !row.change);
  const scopes = (rows: CapabilityRow[]) => [...new Set(rows.map((row) => row.scope))];
  return (
    <>
      <Group
        title="Changes"
        anchor="changes"
        description={
          <>
            Each change is sent to Jira once. Optional for a scoped token: <ScopeNames scopes={scopes(changes)} />, only while changes are on.
          </>
        }
      >
        <Switch label="Allow changes to Jira" checked={behaviour.allowJiraChanges} onChange={setAllowJiraChanges} />
        {changes.map((row) => (
          <Row key={row.capability} label={row.label} description={<Described row={row} />} disabled={!behaviour.allowJiraChanges}>
            <SwitchInput
              aria-label={row.label}
              checked={!behaviour.disabledChanges.includes(row.capability)}
              disabled={!behaviour.allowJiraChanges}
              onChange={(enabled) => setChange(row, enabled)}
            />
          </Row>
        ))}
      </Group>
      <Group
        title="What Taskboard reads"
        anchor="reads"
        description={
          <>
            Always on: Taskboard reads only these from Jira. Required for a scoped token: <ScopeNames scopes={scopes(reads)} />.
          </>
        }
      >
        {reads.map((row) => (
          <Row key={row.capability} label={row.label} description={<Described row={row} />} />
        ))}
      </Group>
    </>
  );
}

/** The detail, then the scope it needs as Jira names it. */
function Described({ row }: { row: CapabilityRow }) {
  return (
    <>
      {row.detail} · <code className="settings-scope">{row.scope}</code>
    </>
  );
}

function ScopeNames({ scopes }: { scopes: string[] }) {
  return (
    <>
      {scopes.map((scope, index) => (
        <span key={scope}>
          {index > 0 && ", "}
          <code className="settings-scope">{scope}</code>
        </span>
      ))}
    </>
  );
}
