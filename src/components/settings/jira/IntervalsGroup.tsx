import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { api, toAppError, type Settings, type SyncSettings } from "../../../api";
import { SETTINGS_KEYS, useSettings } from "../../../queries/settings";
import { fail } from "../../../state/store";
import { Group } from "../controls";
import { NumberField } from "../fields";
import { fieldErrorOf, type FieldErrors } from "../useBusyAction";

type IntervalField = "focusedMinutes" | "backgroundMinutes" | "keepDoneDays";

/** The bounds Rust checks (`SyncSettings::validate`). */
const FIELDS: { field: IntervalField; label: string; suffix: string; min: number; max: number }[] = [
  { field: "focusedMinutes", label: "While Taskboard is in front", suffix: "minutes", min: 1, max: 60 },
  { field: "backgroundMinutes", label: "In the background", suffix: "minutes", min: 5, max: 120 },
  { field: "keepDoneDays", label: "Keep done issues", suffix: "days", min: 7, max: 365 },
];

/** Sync → Intervals: how often Taskboard asks Jira for changes, and how long done issues stay. */
export function IntervalsGroup() {
  const client = useQueryClient();
  const { data: settings } = useSettings();
  const [errors, setErrors] = useState<FieldErrors>({});
  if (!settings) return null;
  const { sync } = settings;

  const save = (patch: Partial<SyncSettings>) => {
    api.syncSave({ ...sync, ...patch }).then(
      (saved) => {
        setErrors({});
        client.setQueryData<Settings>(SETTINGS_KEYS.settings, (current) => current && { ...current, sync: saved });
        void client.invalidateQueries({ queryKey: SETTINGS_KEYS.scope });
      },
      (e: unknown) => {
        const err = toAppError(e);
        const fields = fieldErrorOf(err);
        if (fields) setErrors(fields);
        else fail(err, "Could not save the sync settings");
      },
    );
  };

  return (
    <Group title="Intervals" anchor="intervals" description="How often Taskboard asks Jira for changes.">
      {FIELDS.map(({ field, ...props }) => (
        <NumberField
          key={field}
          {...props}
          value={sync[field]}
          error={errors[`sync.${field}`] ?? errors[field]}
          onCommit={(value) => save({ [field]: value })}
        />
      ))}
    </Group>
  );
}
