import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "../../../api";
import { FIELD_CHOICES_KEY, useFieldChoices } from "../../../queries/views";
import { fail } from "../../../state/store";
import { Group } from "../controls";
import { SearchField } from "../SearchField";
import { type MatrixRow, SettingsMatrix } from "../SettingsMatrix";

const COLUMNS = [
  { key: "list", header: "List" },
  { key: "detail", header: "Detail" },
];

/** Every custom field of the site against where it shows in every Space; rows keep their order while ticked. */
export function CustomFieldsPage() {
  const client = useQueryClient();
  const { data: fields } = useFieldChoices();
  const [search, setSearch] = useState("");
  const needle = search.trim().toLowerCase();
  const shown = (fields ?? []).filter((field) => !needle || field.name.toLowerCase().includes(needle));

  const toggle = (fieldId: string, place: string, on: boolean) => {
    const field = shown.find((f) => f.id === fieldId);
    if (!field) return;
    void api.fieldChoiceSet(field.id, place === "list" ? on : field.inList, place === "detail" ? on : field.inDetail).then(
      (saved) => client.setQueryData(FIELD_CHOICES_KEY, saved),
      (e: unknown) => fail(e, `Could not change ${field.name}`),
    );
  };
  const rows: MatrixRow[] = shown.map((field) => ({
    key: field.id,
    header: (
      <>
        <span className="team-row__name">{field.name}</span>
        <span className="settings-matrix__kind">{field.kind}</span>
      </>
    ),
    cells: [
      { checked: field.inList, label: `${field.name} in the list` },
      { checked: field.inDetail, label: `${field.name} in the Issue view` },
    ],
  }));

  return (
    <Group title="Custom fields" anchor="fields" description="Pick where each Jira field shows. Fields without a box are hidden and never downloaded. A Space can hide a field again in its settings.">
      {fields && fields.length === 0 ? (
        <div className="settings-row">
          <span className="settings-empty">Custom fields appear after the first sync.</span>
        </div>
      ) : (
        <SearchField label="Search fields" value={search} onChange={setSearch} />
      )}
      {needle && shown.length === 0 && (
        <div className="settings-row">
          <span className="settings-empty">No field matches “{search.trim()}”.</span>
        </div>
      )}
      {rows.length > 0 && <SettingsMatrix corner="Field" columns={COLUMNS} rows={rows} onToggle={toggle} />}
    </Group>
  );
}
