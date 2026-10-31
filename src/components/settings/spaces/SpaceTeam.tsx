import { useQueryClient } from "@tanstack/react-query";
import { Search } from "lucide-react";
import { useState } from "react";
import { api, type TeamMember } from "../../../api";
import { spaceTeamKey, useSpaceTeam } from "../../../queries/views";
import { fail } from "../../../state/store";
import { Avatar } from "../../issue/Avatar";
import { Group, SwitchInput } from "../controls";

/** Lists with more people than this get a search field. */
const SEARCH_FROM = 8;

/** Everyone with activity in the Space's issues; a switched-off person isn't offered to assign or filter by. */
export function SpaceTeam({ projectId }: { projectId: string }) {
  const client = useQueryClient();
  const { data: team = [] } = useSpaceTeam(projectId);
  const [search, setSearch] = useState("");
  const needle = search.trim().toLowerCase();
  const shown = team.filter((member) => !needle || member.person.name.toLowerCase().includes(needle));

  const setActive = (member: TeamMember, active: boolean) =>
    void api.spaceSetMember(projectId, member.person.accountId, active).then(
      (saved) => client.setQueryData(spaceTeamKey(projectId), saved),
      (e: unknown) => fail(e, `Could not change ${member.person.name}`),
    );

  return (
    <Group
      title="Team"
      anchor="team"
      description="Everyone who is assigned, reported, commented or changed something in this Space's issues. Switch off people who left the team: they're no longer offered to assign issues to or to filter by."
    >
      {team.length === 0 && (
        <div className="settings-row">
          <span className="settings-empty">People appear after the first sync.</span>
        </div>
      )}
      {team.length > SEARCH_FROM && (
        <div className="settings-row status-search">
          <label className="columns-popover__search status-search__field">
            <Search size={13} strokeWidth={2} aria-hidden="true" />
            <input aria-label="Search people" placeholder="Search people" value={search} onChange={(e) => setSearch(e.target.value)} />
          </label>
        </div>
      )}
      {needle && shown.length === 0 && (
        <div className="settings-row">
          <span className="settings-empty">Nobody matches “{search.trim()}”.</span>
        </div>
      )}
      {shown.map((member) => (
        <div key={member.person.accountId} className="settings-row status-row">
          <Avatar person={member.person} size={22} />
          <span className="team-row__name">{member.person.name}</span>
          <span className="status-row__count">{assignedText(member.assigned)}</span>
          <span className="status-row__spacer" />
          <SwitchInput aria-label={`${member.person.name} is on the team`} checked={member.active} onChange={(on) => setActive(member, on)} />
        </div>
      ))}
    </Group>
  );
}

function assignedText(count: number): string {
  if (count === 0) return "No open issues";
  return count === 1 ? "1 open issue" : `${count} open issues`;
}
