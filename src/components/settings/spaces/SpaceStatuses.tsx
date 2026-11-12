import { useQueryClient } from "@tanstack/react-query";
import { api, type StatusStyleRow, type Style } from "../../../api";
import { STATUS_STYLES_KEY, useStatusStyles } from "../../../queries/views";
import { fail } from "../../../state/store";
import { Group } from "../controls";
import { StatusStyleGroups, type StyledStatus } from "../StatusStyleGroups";

/** A Space's statuses by the style they show here, with their issue counts. */
export function SpaceStatuses({ projectId }: { projectId: string }) {
  const client = useQueryClient();
  const { data: rows = [] } = useStatusStyles(projectId);

  const choose = (status: StyledStatus, style: Style | null) =>
    void api.statusStyleSet(projectId, status.id, style).then(
      (saved) => client.setQueryData([...STATUS_STYLES_KEY, projectId], saved),
      (e: unknown) => fail(e, `Could not change ${status.name}`),
    );

  return (
    <Group title="Statuses" description="The statuses this Space's issues are in, in the style each one shows in here. Default is the style chosen in Settings → Statuses.">
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

function styled(row: StatusStyleRow): StyledStatus {
  const fallback = row.app ?? row.detected;
  return {
    id: row.id,
    name: row.name,
    style: row.chosen ?? fallback,
    chosen: row.chosen,
    fallback: { word: "Default", style: fallback },
    note: <span className="status-row__count">{row.issues === 1 ? "1 issue" : `${row.issues} issues`}</span>,
  };
}
