import type { ActivityFilter } from "../api";
import { useStore } from "../state/store";
import type { Command } from "./registry";

/** The Issue view's activity filters: the segmented buttons and their commands. */
export const ACTIVITY_FILTERS: { id: ActivityFilter; label: string; command: string; title: string; key: string }[] = [
  { id: "all", label: "All", command: "activity.all", title: "Show all activity", key: "Alt+1" },
  { id: "comments", label: "Comments", command: "activity.comments", title: "Show only comments", key: "Alt+2" },
  { id: "changes", label: "Changes", command: "activity.changes", title: "Show only changes", key: "Alt+3" },
];

export const ACTIVITY_KEY_COMMANDS: Command[] = ACTIVITY_FILTERS.map(({ id, command, title, key }) => ({
  id: command,
  title,
  scope: "main",
  defaultBinding: key,
  when: () => useStore.getState().issueView !== null,
  run: () => useStore.getState().setActivityFilter(id),
}));
