import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { api, type SpaceNode, type TeamCell, type TeamRow } from "../../../api";
import { useSpaces } from "../../../queries/sidebar";
import { TEAM_MATRIX_KEY, useTeamMatrix } from "../../../queries/views";
import { fail } from "../../../state/store";
import { Avatar } from "../../issue/Avatar";
import { SpaceTile } from "../../SpaceTile";
import { Group } from "../controls";
import { SearchField } from "../SearchField";
import { type MatrixCell, type MatrixColumn, type MatrixRow, SettingsMatrix } from "../SettingsMatrix";

/** Every person with activity in a tracked Space against every Space: who is on which team. Rows keep their order
 *  while boxes are ticked. */
export function TeamPage() {
  const client = useQueryClient();
  const { data: matrix } = useTeamMatrix();
  const { data: spaces = [] } = useSpaces();
  const [search, setSearch] = useState("");
  const needle = search.trim().toLowerCase();
  const columns = (matrix?.spaces ?? []).map((id) => spaces.find((space) => space.projectId === id)).filter((space): space is SpaceNode => !!space);
  const people = (matrix?.people ?? []).filter((row) => !needle || row.person.name.toLowerCase().includes(needle));

  const set = (row: TeamRow, space: SpaceNode, active: boolean) =>
    void api.spaceSetMember(space.projectId, row.person.accountId, active).then(
      (saved) => client.setQueryData(TEAM_MATRIX_KEY, saved),
      (e: unknown) => fail(e, `Could not change ${row.person.name} in ${space.name}`),
    );

  const matrixColumns: MatrixColumn[] = columns.map((space) => ({
    key: space.projectId,
    title: space.name,
    header: (
      <>
        <SpaceTile space={space} size={20} />
        <span className="mono">{space.key}</span>
      </>
    ),
  }));
  const cellIndex = columns.map((space) => matrix?.spaces.indexOf(space.projectId) ?? -1);
  const matrixRows: MatrixRow[] = people.map((row) => ({
    key: row.person.accountId,
    header: (
      <>
        <Avatar person={row.person} size={20} />
        <span className="team-row__name">{row.person.name}</span>
      </>
    ),
    cells: columns.map((space, i) => teamCell(row, space, row.cells[cellIndex[i]])),
  }));
  const toggle = (accountId: string, projectId: string, active: boolean) => {
    const row = people.find((r) => r.person.accountId === accountId);
    const space = columns.find((c) => c.projectId === projectId);
    if (row && space) set(row, space, active);
  };

  return (
    <Group
      description="Who is on each Space's team: offered to assign issues to, as a person filter and as a drop target. Everyone ever assigned an issue of a Space is on its team unless you switch them off; anyone else who reported, commented or changed something there joins only when you switch them on."
    >
      {matrix && matrix.people.length === 0 ? (
        <div className="settings-row">
          <span className="settings-empty">People appear after the first sync.</span>
        </div>
      ) : (
        <SearchField label="Search people" value={search} onChange={setSearch} />
      )}
      {needle && people.length === 0 && (
        <div className="settings-row">
          <span className="settings-empty">Nobody matches “{search.trim()}”.</span>
        </div>
      )}
      {people.length > 0 && (
        <SettingsMatrix corner="Person" columns={matrixColumns} rows={matrixRows} onToggle={toggle} />
      )}
    </Group>
  );
}

function teamCell(row: TeamRow, space: SpaceNode, cell: TeamCell | undefined): MatrixCell {
  return { checked: cell?.active ?? false, label: `${row.person.name} on the ${space.name} team`, hint: cellHint(cell), quiet: !cell?.involved };
}

function cellHint(cell: TeamCell | undefined): string {
  if (cell?.assignee) return "Assigned issues here: on the team unless switched off";
  if (cell?.involved) return "Reported, commented or changed something here";
  return "No activity here";
}
