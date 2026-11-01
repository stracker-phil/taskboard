import { api, type Priorities } from "../api";
import { PriorityFlag } from "../components/issue/PriorityFlag";
import { openMenu, type MenuAt, type MenuEntry } from "../components/menu";
import { attempt } from "../state/requests";
import { menuAnchor } from "./menuAnchor";
import { runWrite } from "./runWrite";

interface Issue {
  id: string;
  key: string;
}

/** Asks Jira which priorities the issue's edit screen offers, then offers each of them. */
export function openPriorityMenu(issue: Issue, at: MenuAt) {
  void attempt(api.issuePriorities(issue.id), "Could not load the priorities").then((priorities) => {
    if (priorities) openMenu(priorityEntries(issue, priorities), at, `Priority of ${issue.key}`);
  });
}

function priorityEntries(issue: Issue, { options, current }: Priorities): MenuEntry[] {
  return [
    { caption: `Set ${issue.key} to` },
    ...options.map((priority) => ({
      text: priority.name,
      icon: <PriorityFlag level={priority.level} />,
      checked: priority.name === current,
      run: () => runWrite(issue.id, api.issueWrite(issue.id, { kind: "priority", name: priority.name }), "Could not change the priority"),
    })),
  ];
}

export const priorityMenuAnchor = (issueId: string): MenuAt => menuAnchor("data-priority-menu", issueId);
