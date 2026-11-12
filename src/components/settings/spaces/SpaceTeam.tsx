import { api } from "../../../api";
import { useConnection } from "../../../queries/settings";
import { useTeamMatrix } from "../../../queries/views";
import { attempt } from "../../../state/requests";
import { Avatar } from "../../issue/Avatar";
import { Group, Row } from "../controls";

/** The Space's team, read-only; Settings → Team edits it alongside every other Space. */
export function SpaceTeam({ projectId }: { projectId: string }) {
  const { data: matrix } = useTeamMatrix();
  const { data: connection } = useConnection();
  const me = connection?.connection.accountId;
  const column = matrix?.spaces.indexOf(projectId) ?? -1;
  const rows = column < 0 ? [] : (matrix?.people ?? []);
  const members = rows.filter((row) => row.cells[column].active).map((row) => row.person);
  const involved = rows.filter((row) => row.cells[column].involved).length;
  return (
    <Group title="Team" description="Who is offered to assign this Space's issues to, as a person filter and as a drop target.">
      <Row label={members.length === 1 ? "1 person on the team" : `${members.length} people on the team`} description={`${involved} with activity in this Space`}>
        <button type="button" className="btn" onClick={() => void attempt(api.runCommand("app:settings:team"), "Could not open the Team settings")}>
          Edit Team…
        </button>
      </Row>
      {members.length > 0 && (
        <ul className="space-team" aria-label="Team members">
          {members.map((person) => (
            <li key={person.accountId} className="space-team__person">
              <Avatar person={person} size={20} />
              <span className="team-row__name">{person.name}</span>
              {person.accountId === me && <span className="space-team__you">(you)</span>}
            </li>
          ))}
        </ul>
      )}
    </Group>
  );
}
