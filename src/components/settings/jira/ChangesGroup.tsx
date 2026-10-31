import { api } from "../../../api";
import { useSettings } from "../../../queries/settings";
import { attempt } from "../../../state/requests";
import { Group, Switch } from "../controls";

export function ChangesGroup() {
  const { data: settings } = useSettings();
  if (!settings) return null;
  const { behaviour } = settings;
  const setAllowJiraChanges = (allowJiraChanges: boolean) =>
    void attempt(api.behaviourSet({ ...behaviour, allowJiraChanges }), "Could not save the setting");
  return (
    <Group title="Changes" anchor="changes">
      <Switch
        label="Allow changes to Jira"
        description="Assign issues, change their status and comment from Taskboard. Each change is sent once; a scoped token needs write:jira-work."
        checked={behaviour.allowJiraChanges}
        onChange={setAllowJiraChanges}
      />
    </Group>
  );
}
