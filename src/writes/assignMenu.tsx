import { api, type Assignees } from "../api";
import { Avatar } from "../components/issue/Avatar";
import { openMenu, type MenuAt, type MenuEntry } from "../components/menu";
import { attempt } from "../state/requests";
import { runWrite } from "./runWrite";

interface Issue {
  id: string;
  key: string;
}

/** Asks Rust who is on the issue's team, then offers each of them and Unassign. */
export function openAssignMenu(issue: Issue, at: MenuAt) {
  void attempt(api.issueAssignees(issue.id), "Could not load the team").then((assignees) => {
    if (assignees) openMenu(assignEntries(issue, assignees), at, `Assignee of ${issue.key}`);
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

/** Where A opens the assignee menu: under the issue's assignee when one is on screen, else under its row or card,
 *  else near the top of the window. */
export function assignMenuAnchor(issueId: string): MenuAt {
  const id = CSS.escape(issueId);
  return (
    document.querySelector<HTMLElement>(`[data-assign-menu="${id}"]`) ??
    document.querySelector<HTMLElement>(`[data-issue-id="${id}"]`) ?? { x: window.innerWidth / 2, y: 120 }
  );
}
