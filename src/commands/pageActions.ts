import { useEffect, useRef } from "react";
import type { Command } from "./registry";

/** What the page can do right now, by command id; components add theirs while they are on screen. */
const actions = new Map<string, () => void>();

/** Offers `run` under `id` while mounted; the latest registration wins. */
export function usePageAction(id: string, run: (() => void) | null) {
  const latest = useRef(run);
  useEffect(() => {
    latest.current = run;
  });
  const offered = run !== null;
  useEffect(() => {
    if (!offered) return;
    const action = () => latest.current?.();
    actions.set(id, action);
    return () => {
      if (actions.get(id) === action) actions.delete(id);
    };
  }, [id, offered]);
}

/** Runs a page action, e.g. from a menu item; false when nothing on screen offers it. */
export function runPageAction(id: string): boolean {
  const action = actions.get(id);
  action?.();
  return action !== undefined;
}

const action = (id: string, title: string, defaultBinding?: string): Command => ({
  id,
  title,
  scope: "main",
  defaultBinding,
  when: () => actions.has(id),
  run: () => void runPageAction(id),
});

/** Every page action as a command: rebindable in Settings → Shortcuts, listed in the palette and the menus. */
export const PAGE_ACTION_COMMANDS: Command[] = [
  action("list.new", "New Filter List", "Mod+N"),
  action("view.new", "New View", "Mod+T"),
  action("view.next", "Next View", "Ctrl+Tab"),
  action("view.previous", "Previous View", "Ctrl+Shift+Tab"),
  action("view.layout.list", "Show as List", "Mod+Ctrl+1"),
  action("view.layout.board", "Show as Board", "Mod+Ctrl+2"),
  action("view.layout.table", "Show as Table", "Mod+Ctrl+3"),
  action("toolbar.me", "Only My Issues", "Mod+Shift+M"),
  action("toolbar.search", "Filter This View", "Mod+F"),
  action("toolbar.filter", "Edit Filter…", "Mod+Alt+F"),
  action("toolbar.columns", "Columns…", "Mod+Alt+C"),
  action("toolbar.group", "Group and Sort…", "Mod+Alt+G"),
  action("go.everything", "Go to Everything", "Mod+Shift+E"),
  action("go.recent", "Go to Recently Changed", "Mod+Shift+U"),
  action("go.pinned", "Go to Pinned", "Mod+Shift+P"),
  action("recent.markAllSeen", "Mark All Seen", "Mod+Shift+A"),
];
