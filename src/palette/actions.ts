import { api, type GroupBy, type IssueHit, type Nav, type SidebarTree } from "../api";
import { allCommands } from "../commands/registry";
import { GROUP_NAMES } from "../shell/toolbar/groupMenu";
import { attempt, runBusCommand } from "../state/requests";

/** A palette action: a label, an optional key hint, and its work. */
export interface PaletteAction {
  id: string;
  label: string;
  /** Shown at the right ("⌘R"). */
  hint?: string;
  run: () => void | Promise<void>;
}

interface Context {
  text: string;
  /** The active issue, or the first one found. */
  issue: IssueHit | undefined;
  openInJira: ((key: string) => void) | null;
  tree: SidebarTree | undefined;
  nav: Nav | undefined;
  /** Opens a level once it exists. */
  go: (listId: number) => Promise<void>;
  viewId: number | null;
}

const NEW_LIST_PREFIX = "Search: ";

/** The current Space, else Everything: where a saved search goes. */
function saveTarget(tree: SidebarTree | undefined, nav: Nav | undefined): number | undefined {
  const listId = nav?.location?.listId;
  const space = tree?.spaces.find((s) => s.listId === listId || s.lists.some((l) => l.id === listId));
  return space?.listId ?? tree?.everything.id;
}

/** "Save as a filter list": a new list with `text contains` under the current Space or Everything, opened. */
export async function saveSearch(text: string, tree: SidebarTree | undefined, nav: Nav | undefined, go: (listId: number) => Promise<void>) {
  const parent = saveTarget(tree, nav);
  if (parent === undefined) return;
  const filter = { conditions: [{ field: "text", op: "contains" as const, value: text }] };
  const id = await attempt(api.listCreate(parent, `${NEW_LIST_PREFIX}${text}`, filter), "Could not save the search");
  if (id !== null) await go(id);
}

const fold = (text: string) => text.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();

/** The palette's own actions first, then every titled command that can run now; all filtered by the text. */
export function paletteActions(ctx: Context): PaletteAction[] {
  const text = ctx.text.trim();
  const own: PaletteAction[] = [];
  const issue = ctx.issue;
  const openInJira = ctx.openInJira;
  if (issue && openInJira) own.push({ id: "jira", label: `Open ${issue.key} in Jira`, hint: "⌘↩", run: () => openInJira(issue.key) });
  if (text) own.push({ id: "save", label: `Save “${text}” as a filter list`, hint: "⌘S", run: () => saveSearch(text, ctx.tree, ctx.nav, ctx.go) });
  own.push({ id: "refresh", label: "Refresh now", hint: "⌘R", run: () => runBusCommand("sync:refresh", "Could not refresh") });
  own.push({ id: "full", label: "Full sync", run: () => runBusCommand("sync:full", "Could not start a full sync") });
  const viewId = ctx.viewId;
  if (viewId !== null) {
    for (const groupBy of Object.keys(GROUP_NAMES) as GroupBy[]) {
      own.push({
        id: `group.${groupBy}`,
        label: `Group by ${GROUP_NAMES[groupBy]}`,
        run: () => void attempt(api.viewSetGroup(viewId, groupBy), "Could not change the grouping"),
      });
    }
  }
  const commands: PaletteAction[] = allCommands()
    .filter((c) => c.scope !== "global" && c.id !== "palette.open" && (c.when?.() ?? true))
    .map((c) => ({ id: `command.${c.id}`, label: c.title, run: c.run }));
  const needle = fold(text);
  // The issue and save actions belong to the text; the rest are found by name.
  const matches = (a: PaletteAction) => a.id === "jira" || a.id === "save" || !needle || fold(a.label).includes(needle);
  const seen = new Set<string>();
  return [...own, ...commands].filter((a) => {
    if (!matches(a) || seen.has(a.label)) return false;
    seen.add(a.label);
    return true;
  });
}
