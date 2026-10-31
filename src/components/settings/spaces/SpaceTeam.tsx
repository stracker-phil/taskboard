import { api } from "../../../api";
import { useTeamMatrix } from "../../../queries/views";
import { attempt } from "../../../state/requests";
import { Group, Row } from "../controls";

/** How many people are on the Space's team; Settings → Team edits it alongside every other Space. */
export function SpaceTeam({ projectId }: { projectId: string }) {
  const { data: matrix } = useTeamMatrix();
  const column = matrix?.spaces.indexOf(projectId) ?? -1;
  const cells = column < 0 ? [] : (matrix?.people ?? []).map((row) => row.cells[column]);
  const active = cells.filter((cell) => cell.active).length;
  const involved = cells.filter((cell) => cell.involved).length;
  return (
    <Group title="Team" anchor="team" description="Who is offered to assign this Space's issues to, as a person filter and as a drop target.">
      <Row label={active === 1 ? "1 person on the team" : `${active} people on the team`} description={`${involved} with activity in this Space`}>
        <button type="button" className="btn" onClick={() => void attempt(api.runCommand("app:settings:team"), "Could not open the Team settings")}>
          Edit Team…
        </button>
      </Row>
    </Group>
  );
}
