import { api, type CapabilityRow } from "../../../api";
import { useCapabilities, useChangePermissions, useSettings } from "../../../queries/settings";
import { attempt } from "../../../state/requests";
import { Group, Row, Switch, SwitchInput } from "../controls";

/** Changes to Jira on or off, then each kind of change on its own, each with the token scope it needs. The list
 *  comes from the gateway's own requests (`jira::capability`), so it is what the app does. */
export function ChangesGroup() {
  const { data: settings } = useSettings();
  const { data: capabilities = [] } = useCapabilities();
  const { data: notAllowed = [] } = useChangePermissions(settings?.behaviour.allowJiraChanges ?? false);
  if (!settings) return null;
  const { behaviour } = settings;
  const setAllowJiraChanges = (allowJiraChanges: boolean) =>
    void attempt(api.behaviourSet({ ...behaviour, allowJiraChanges }), "Could not save the setting");
  const setChange = (row: CapabilityRow, enabled: boolean) =>
    void attempt(api.jiraCapabilitySet(row.capability, enabled), `Could not change “${row.label}”`);
  const changes = capabilities.filter((row) => row.change);
  const notAllowedIn = (row: CapabilityRow) =>
    behaviour.allowJiraChanges ? (notAllowed.find((n) => n.capability === row.capability)?.projects ?? []) : [];
  return (
    <Group
      title="Changes"
      anchor="changes"
      description="Each change is sent to Jira once."
    >
      <Switch label="Allow changes to Jira" checked={behaviour.allowJiraChanges} onChange={setAllowJiraChanges} />
      {changes.map((row) => (
        <Row
          key={row.capability}
          label={row.label}
          description={<Described row={row} notAllowedIn={notAllowedIn(row)} />}
          disabled={!behaviour.allowJiraChanges}
        >
          <SwitchInput
            aria-label={row.label}
            checked={!behaviour.disabledChanges.includes(row.capability)}
            disabled={!behaviour.allowJiraChanges}
            onChange={(enabled) => setChange(row, enabled)}
          />
        </Row>
      ))}
    </Group>
  );
}

/** Read-only capabilities, each with the token scope it needs; no switches. */
export function ReadsGroup() {
  const { data: capabilities } = useCapabilities();
  if (!capabilities) return null;
  const reads = capabilities.filter((row) => !row.change);
  return (
    <Group
      title="What Taskboard reads"
      anchor="reads"
      description="Always on: Taskboard reads only these from Jira."
    >
      {reads.map((row) => (
        <Row key={row.capability} label={row.label} description={<Described row={row} />} />
      ))}
    </Group>
  );
}

/** The detail and its token scope; for a change, the projects where Jira doesn't allow it. */
function Described({ row, notAllowedIn = [] }: { row: CapabilityRow; notAllowedIn?: string[] }) {
  return (
    <>
      {row.detail} · <code className="settings-scope">{row.scope}</code>
      {notAllowedIn.length > 0 && (
        <span className="settings-not-allowed">Jira doesn't allow it in {notAllowedIn.join(", ")}</span>
      )}
    </>
  );
}
