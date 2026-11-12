import { useQueryClient } from "@tanstack/react-query";
import { api, type FieldSetting, type SpaceConfig } from "../../../api";
import { SPACE_CONFIG_KEY } from "../../../queries/views";
import { attempt } from "../../../state/requests";
import { Group } from "../controls";
import { type MatrixRow, SettingsMatrix } from "../SettingsMatrix";

const COLUMNS = [
  { key: "list", header: "List" },
  { key: "detail", header: "Detail" },
];

/** Unticking a box hides the field in this Space only. */
export function SpaceCustomFields({ config }: { config: SpaceConfig }) {
  const client = useQueryClient();
  const set = (field: FieldSetting, inList: boolean, inDetail: boolean) =>
    void attempt(
      api.spaceSetField(config.projectId, field.id, inList, inDetail).then((saved) => client.setQueryData([...SPACE_CONFIG_KEY, config.projectId], saved)),
      `Could not change ${field.name}`,
    );
  const toggle = (fieldId: string, place: string, on: boolean) => {
    const field = config.fields.find((f) => f.id === fieldId);
    if (field) set(field, place === "list" ? on : field.inList, place === "detail" ? on : field.inDetail);
  };
  const rows: MatrixRow[] = config.fields.map((field) => ({
    key: field.id,
    header: <span className="team-row__name">{field.name}</span>,
    cells: [
      field.appList ? { checked: field.inList, label: `${field.name} in this Space's lists` } : undefined,
      field.appDetail ? { checked: field.inDetail, label: `${field.name} in this Space's Issue view` } : undefined,
    ],
  }));

  return (
    <Group title="Custom fields" description="Where the fields picked in Settings → Custom fields show in this Space: as a column and filter choice, in the Issue view.">
      {rows.length === 0 ? (
        <div className="settings-row">
          <span className="settings-empty">No fields are shown. Pick fields in Settings → Custom fields.</span>
          <button type="button" className="btn" onClick={() => void attempt(api.runCommand("app:settings:fields"), "Could not open the Custom fields settings")}>
            Custom Fields…
          </button>
        </div>
      ) : (
        <SettingsMatrix corner="Field" columns={COLUMNS} rows={rows} onToggle={toggle} />
      )}
    </Group>
  );
}
