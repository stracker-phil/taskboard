import { api, type TransitionOption } from "../api";
import { StatusSquare } from "../components/issue/StatusSquare";
import { openMenu, type MenuAt, type MenuEntry, type MenuItem } from "../components/menu";
import { attempt } from "../state/requests";
import { menuAnchor } from "./menuAnchor";
import { runWrite } from "./runWrite";

interface Issue {
  id: string;
  key: string;
}

/** Asks Jira for the issue's moves, then shows one row per move. */
export function openStatusMenu(issue: Issue, at: MenuAt, openInJira: ((key: string) => void) | null) {
  void attempt(api.issueTransitions(issue.id), "Could not load the statuses").then((moves) => {
    if (moves) openMoveMenu(issue, moves, at, openInJira);
  });
}

/** A menu of these moves (a drop on a status type that several statuses can follow). */
export function openMoveMenu(issue: Issue, moves: TransitionOption[], at: MenuAt, openInJira: ((key: string) => void) | null) {
  openMenu(statusEntries(issue, moves, openInJira), at, `Status of ${issue.key}`, { search: "Search statuses" });
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

export const statusMenuAnchor = (issueId: string): MenuAt => menuAnchor("data-status-menu", issueId);
