import { useQueryClient } from "@tanstack/react-query";
import { type AppStatusRow, api, type Style } from "../../../api";
import { APP_STATUS_STYLES_KEY, useAppStatusStyles } from "../../../queries/views";
import { fail } from "../../../state/store";
import { Group } from "../controls";
import { StatusStyleGroups, type StyledStatus } from "../StatusStyleGroups";

/** Every status of the tracked projects by the style it shows in every Space that doesn't choose its own. */
export function StatusesPage() {
  const client = useQueryClient();
  const { data: rows = [] } = useAppStatusStyles();

  const choose = (status: StyledStatus, style: Style | null) =>
    void api.statusStyleSetApp(status.id, style).then(
      (saved) => client.setQueryData(APP_STATUS_STYLES_KEY, saved),
      (e: unknown) => fail(e, `Could not change ${status.name}`),
    );

  return (
    <Group
      title="Statuses"
      anchor="statuses"
      description="The style each status shows in, in every Space. Automatic picks one from the Jira category and the name; a Space can choose its own in its settings."
    >
      {rows.length === 0 ? (
        <div className="settings-row">
          <span className="settings-empty">Statuses appear after the first sync.</span>
        </div>
      ) : (
        <StatusStyleGroups statuses={rows.map(styled)} choose={choose} />
      )}
    </Group>
  );
}

function styled(row: AppStatusRow): StyledStatus {
  return { id: row.id, name: row.name, style: row.chosen ?? row.detected, chosen: row.chosen, fallback: { word: "Automatic", style: row.detected } };
}
