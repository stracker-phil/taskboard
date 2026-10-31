import { api, type CapabilityRow } from "../../../api";
import { useCapabilities, useSettings } from "../../../queries/settings";
import { attempt } from "../../../state/requests";
import { Group, Row, Switch, SwitchInput } from "../controls";

/** Changes to Jira on or off, then each kind of change on its own, then what Taskboard reads. The list comes from the
 *  gateway's own requests (`jira::capability`), so it is what the app does. */
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
  return (
    <>
      <Group title="Changes" anchor="changes" description="Each change is sent to Jira once. A scoped token needs write:jira-work.">
        <Switch label="Allow changes to Jira" checked={behaviour.allowJiraChanges} onChange={setAllowJiraChanges} />
        {changes.map((row) => (
          <Row key={row.capability} label={row.label} description={row.detail} disabled={!behaviour.allowJiraChanges}>
            <SwitchInput
              aria-label={row.label}
              checked={!behaviour.disabledChanges.includes(row.capability)}
              disabled={!behaviour.allowJiraChanges}
              onChange={(enabled) => setChange(row, enabled)}
            />
          </Row>
        ))}
      </Group>
      <Group title="What Taskboard reads" anchor="reads" description="Always on: Taskboard reads only these from Jira.">
        {reads.map((row) => (
          <Row key={row.capability} label={row.label} description={row.detail} />
        ))}
      </Group>
    </>
  );
}
