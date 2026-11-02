import { api, type IssueChange, type Person } from "../api";
import { Avatar } from "../components/issue/Avatar";
import { PriorityFlag } from "../components/issue/PriorityFlag";
import { StatusSquare } from "../components/issue/StatusSquare";
import { openMenu, type MenuAt, type MenuEntry } from "../components/menu";
import { attempt } from "../state/requests";
import { useStore } from "../state/store";

const issues = (n: number) => (n === 1 ? "1 issue" : `${n} issues`);

/** Sends the changes one after another and says in one toast what happened, with "Change back" for those that
 *  changed (a new change, never an undo). */
export async function runBulk(changes: IssueChange[], what: string, total: number) {
  const outcome = await attempt(api.issuesWriteMany(changes, what, total), "Could not change the issues");
  if (!outcome) return;
  const kind = outcome.refused === 0 ? "success" : outcome.changed === 0 ? "error" : "info";
  const reverse = outcome.reverse;
  const action = reverse ? { label: "Change back", run: () => void runBulk(reverse, "Changed back", reverse.length) } : undefined;
  useStore.getState().toast(kind, outcome.done, action);
}

/** A menu that waits for its choices: `busy` marks its button while Jira answers. */
async function openWhenLoaded<T>(load: Promise<T>, failure: string, busy: (on: boolean) => void, open: (loaded: T) => void) {
  busy(true);
  const loaded = await attempt(load, failure);
  busy(false);
  if (loaded) open(loaded);
}

/** The statuses every picked issue can move to; an issue already there is left as it is. */
export function openBulkStatusMenu(ids: string[], at: MenuAt, busy: (on: boolean) => void) {
  void openWhenLoaded(api.issuesBulkStatuses(ids), "Could not load the statuses", busy, (statuses) => {
    const entries: MenuEntry[] = statuses.map((status) => ({
      text: status.statusName,
      icon: <StatusSquare style={status.style} />,
      run: () => void runBulk(status.changes, `Moved to ${status.statusName}`, ids.length),
    }));
    openMenu([{ caption: `Move ${issues(ids.length)} to` }, ...(entries.length > 0 ? entries : [{ text: "No status all of them can move to", enabled: false }])], at, "Status of the picked issues", { search: "Search statuses" });
  });
}

/** The priorities every picked issue's edit screen offers. */
export function openBulkPriorityMenu(ids: string[], at: MenuAt, busy: (on: boolean) => void) {
  void openWhenLoaded(api.issuesBulkPriorities(ids), "Could not load the priorities", busy, (priorities) => {
    const entries: MenuEntry[] = priorities.map((priority) => ({
      text: priority.name,
      icon: <PriorityFlag level={priority.level} />,
      run: () => void runBulk(ids.map((issueId) => ({ issueId, change: { kind: "priority", name: priority.name } })), `Set to ${priority.name}`, ids.length),
    }));
    openMenu([{ caption: `Set ${issues(ids.length)} to` }, ...(entries.length > 0 ? entries : [{ text: "No priority all of them offer", enabled: false }])], at, "Priority of the picked issues");
  });
}

/** Me and whoever is on the team of every picked issue's Space, then Unassigned. */
export function openBulkAssignMenu(ids: string[], at: MenuAt, busy: (on: boolean) => void) {
  void openWhenLoaded(api.issuesBulkAssignees(ids), "Could not load the team", busy, (people) => {
    const assign = (person: Person | null) => () =>
      void runBulk(ids.map((issueId) => ({ issueId, change: { kind: "assign", accountId: person?.accountId ?? null } })), person ? `Assigned to ${person.name}` : "Unassigned", ids.length);
    openMenu(
      [
        { caption: `Assign ${issues(ids.length)} to` },
        ...people.map((person) => ({ text: person.name, icon: <Avatar person={person} size={18} />, run: assign(person) })),
        "separator",
        { text: "Unassigned", run: assign(null) },
      ],
      at,
      "Assignee of the picked issues",
      { search: "Search people" },
    );
  });
}
