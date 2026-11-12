import { useId } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { api, type Settings } from "../../../api";
import { SETTINGS_KEYS, useIssueTypes, useSettings } from "../../../queries/settings";
import { ValuePicker } from "../../../shell/toolbar/ValuePicker";
import { fail } from "../../../state/store";
import { Group, Row } from "../controls";

/** Sync → Issue types: types Taskboard doesn't copy at all, whatever the scope. */
export function IssueTypesGroup() {
  const client = useQueryClient();
  const id = useId();
  const { data: settings } = useSettings();
  const { data: types = [] } = useIssueTypes();
  if (!settings) return null;
  const { sync } = settings;

  const save = (excludedTypes: string[]) =>
    api.syncSave({ ...sync, excludedTypes }).then(
      (saved) => client.setQueryData<Settings>(SETTINGS_KEYS.settings, (current) => current && { ...current, sync: saved }),
      (e: unknown) => fail(e, "Could not save the issue types"),
    );

  return (
    <Group title="Issue types" anchor="issue-types" description="Types you don't need, such as test management issues.">
      <Row label="Leave out" htmlFor={id} description="Not copied, listed or counted; the copies Taskboard has go at once.">
        <ValuePicker id={id} choices={types.map((name) => ({ id: name, name }))} selected={sync.excludedTypes} placeholder="None" onChange={(values) => void save(values)} />
      </Row>
    </Group>
  );
}
