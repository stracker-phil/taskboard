import { api, type Assignees } from "../api";
import { Avatar } from "../components/issue/Avatar";
import { openMenu, type MenuAt, type MenuEntry } from "../components/menu";
import { attempt } from "../state/requests";
import { menuAnchor } from "./menuAnchor";
import { runWrite } from "./runWrite";

interface Issue {
  id: string;
  key: string;
}

/** Asks Rust who is on the issue's team, then offers each of them and Unassign. */
export function openAssignMenu(issue: Issue, at: MenuAt) {
  void attempt(api.issueAssignees(issue.id), "Could not load the team").then((assignees) => {
    if (assignees) openMenu(assignEntries(issue, assignees), at, `Assignee of ${issue.key}`, { search: "Search people" });
  });
}

function assignEntries(issue: Issue, { people, current }: Assignees): MenuEntry[] {
  const assign = (accountId: string | null) => () =>
    runWrite(issue.id, api.issueWrite(issue.id, { kind: "assign", accountId }), "Could not change the assignee");
  return [
    { caption: `Assign ${issue.key} to` },
    ...people.map((person) => ({
      text: person.name,
      icon: <Avatar person={person} size={18} />,
      checked: person.accountId === current,
      run: assign(person.accountId),
    })),
    "separator",
    { text: "Unassigned", checked: current === null, run: assign(null) },
  ];
}

export const assignMenuAnchor = (issueId: string): MenuAt => menuAnchor("data-assign-menu", issueId);
