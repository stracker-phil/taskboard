import { useQueryClient } from "@tanstack/react-query";
import { Search } from "lucide-react";
import { useState } from "react";
import { api, type SpaceNode, type TeamCell, type TeamRow } from "../../../api";
import { useSpaces } from "../../../queries/sidebar";
import { TEAM_MATRIX_KEY, useTeamMatrix } from "../../../queries/views";
import { fail } from "../../../state/store";
import { Avatar } from "../../issue/Avatar";
import { SpaceTile } from "../../SpaceTile";
import { Group } from "../controls";

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

  return (
    <Group
      title="Team"
      anchor="team"
      description="Who is on each Space's team: offered to assign issues to, as a person filter and as a drop target. Everyone ever assigned an issue of a Space is on its team unless you switch them off; anyone else who reported, commented or changed something there joins only when you switch them on."
    >
      {matrix && matrix.people.length === 0 ? (
        <div className="settings-row">
          <span className="settings-empty">People appear after the first sync.</span>
        </div>
      ) : (
        <div className="settings-row status-search">
          <label className="columns-popover__search status-search__field">
            <Search size={13} strokeWidth={2} aria-hidden="true" />
            <input aria-label="Search people" placeholder="Search people" value={search} onChange={(e) => setSearch(e.target.value)} />
          </label>
        </div>
      )}
      {needle && people.length === 0 && (
        <div className="settings-row">
          <span className="settings-empty">Nobody matches “{search.trim()}”.</span>
        </div>
      )}
      {people.length > 0 && (
        <div className="team-matrix scroller">
          <table>
            <thead>
              <tr>
                <th scope="col" className="team-matrix__person">
                  Person
                </th>
                {columns.map((space) => (
                  <th key={space.projectId} scope="col" title={space.name} className="team-matrix__space">
                    <SpaceTile space={space} size={20} />
                    <span className="mono">{space.key}</span>
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {people.map((row) => (
                <tr key={row.person.accountId}>
                  <th scope="row" className="team-matrix__person">
                    <span className="team-matrix__name">
                      <Avatar person={row.person} size={20} />
                      <span className="team-row__name">{row.person.name}</span>
                    </span>
                  </th>
                  {columns.map((space) => {
                    const cell = row.cells[matrix?.spaces.indexOf(space.projectId) ?? -1];
                    return (
                      <td key={space.projectId} className={cell?.involved ? "team-matrix__cell team-matrix__cell--involved" : "team-matrix__cell"} title={cellHint(cell)}>
                        <input
                          type="checkbox"
                          className="team-matrix__check"
                          aria-label={`${row.person.name} on the ${space.name} team`}
                          checked={cell?.active ?? false}
                          onChange={(e) => set(row, space, e.target.checked)}
                        />
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </Group>
  );
}

function cellHint(cell: TeamCell | undefined): string {
  if (cell?.assignee) return "Assigned issues here: on the team unless switched off";
  if (cell?.involved) return "Reported, commented or changed something here";
  return "No activity here";
}
