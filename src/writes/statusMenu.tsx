import { api, type TransitionOption } from "../api";
import { StatusSquare } from "../components/issue/StatusSquare";
import { openMenu, type MenuAt, type MenuEntry, type MenuItem } from "../components/menu";
import { attempt } from "../state/requests";
import { runWrite } from "./runWrite";

interface Issue {
  id: string;
  key: string;
}

/** Asks Jira for the issue's moves, then shows one row per move. */
export function openStatusMenu(issue: Issue, at: MenuAt, openInJira: ((key: string) => void) | null) {
  void attempt(api.issueTransitions(issue.id), "Could not load the statuses").then((moves) => {
    if (moves) openMenu(statusEntries(issue, moves, openInJira), at, `Status of ${issue.key}`);
  });
}

function statusEntries(issue: Issue, moves: TransitionOption[], openInJira: ((key: string) => void) | null): MenuEntry[] {
  const rows = moves.map((move) => moveRow(issue, move, openInJira));
  return [{ caption: `Move ${issue.key} to` }, ...(rows.length > 0 ? rows : [{ text: "No other status", enabled: false }])];
}

/** A move that needs Jira's screen opens the issue in Jira instead. */
function moveRow(issue: Issue, move: TransitionOption, openInJira: ((key: string) => void) | null): MenuItem {
  const row = { text: move.statusName, icon: <StatusSquare style={move.style} /> };
  if (move.opensJira) {
    return { ...row, hint: "Opens Jira", enabled: openInJira !== null, run: () => openInJira?.(issue.key) };
  }
  const change = { kind: "transition", transitionId: move.id } as const;
  return { ...row, run: () => runWrite(issue.id, api.issueWrite(issue.id, change), "Could not change the status") };
}

/** Where S opens the status menu: under the issue's status pill when one is on screen, else under its row or card,
 *  else near the top of the window. */
export function statusMenuAnchor(issueId: string): MenuAt {
  const id = CSS.escape(issueId);
  return (
    document.querySelector<HTMLElement>(`[data-status-menu="${id}"]`) ??
    document.querySelector<HTMLElement>(`[data-issue-id="${id}"]`) ?? { x: window.innerWidth / 2, y: 120 }
  );
}
