import type { MenuCommand } from "../api";
import { acceleratorLabel, allCommands, bindingFor, bindingLabel, getCommand, type Command } from "../commands/registry";
import type { MenuEntry, MenuItem } from "../components/menu";
import { settingsPages } from "../registry/settingsPages";
import { runBusCommand } from "../state/requests";
import { paletteMenus } from "./menus";

/** A row of the palette's commands. */
export interface PaletteCommand {
  id: string;
  label: string;
  /** Its menu, or the part of the page it comes from. */
  section: string;
  /** Searched with the label and shown after it. */
  detail?: string;
  /** The key that runs it as well ("⌘⇧R"). */
  shortcut?: string;
  /** An option that is on. */
  checked?: boolean;
  run: () => void;
}

const PAGE = "page:";
const SETTINGS_SECTION = "Settings";

const keyOf = (command: Command) => bindingLabel(bindingFor(command.id)) || undefined;

/** What the page offers at one moment. */
export interface PageSnapshot {
  /** The page's commands that could run. */
  runnable: Set<string>;
  /** The page's own menus and the Settings pages. */
  own: PaletteCommand[];
}

/** Taken before the palette is on screen: some commands only run while no dialog or text field has the focus. */
export function snapshotPage(): PageSnapshot {
  return {
    runnable: new Set(allCommands().flatMap((c) => (c.when?.() ?? true ? [c.id] : []))),
    own: [...paletteMenus().flatMap((m) => flatten(m, m.entries())), ...settingsCommands()],
  };
}

/** A menu item runs the page's command of the same id, or a Rust command (through the page's twin when one has a key
 *  of its own); a page command that couldn't run is left out. */
function fromMenu(item: MenuCommand, page: PageSnapshot, twins: Map<string, Command>): PaletteCommand | null {
  const base = { id: item.command, label: item.label, section: item.menu };
  if (item.command.startsWith(PAGE)) {
    const command = getCommand(item.command.slice(PAGE.length));
    if (!command || !page.runnable.has(command.id)) return null;
    return { ...base, shortcut: keyOf(command), run: () => void command.run() };
  }
  const twin = twins.get(item.command);
  if (twin) return { ...base, shortcut: keyOf(twin), run: () => void twin.run() };
  return { ...base, shortcut: acceleratorLabel(item.accelerator) || undefined, run: () => runBusCommand(item.command, `Could not run “${item.label}”`) };
}

/** A menu's rows, submenus flattened: "Group: Sort by › Updated". Captions and separators drop, greyed rows too. */
function flatten(menu: { section: string; title: string }, entries: MenuEntry[], path: string[] = []): PaletteCommand[] {
  return entries.flatMap((entry): PaletteCommand[] => {
    if (entry === "separator" || "caption" in entry) return [];
    const item: MenuItem = entry;
    if (item.enabled === false) return [];
    const at = [...path, item.text];
    if (item.items) return flatten(menu, item.items, at);
    const run = item.run;
    if (!run) return [];
    const label = `${menu.title}: ${at.join(" › ")}`;
    return [{ id: `${menu.section}:${label}`, label, section: menu.section, detail: item.hint, checked: item.checked, run }];
  });
}

/** Each Settings page opens at its own place; its groups are searched too ("token" finds Connection). */
function settingsCommands(): PaletteCommand[] {
  return settingsPages().map((page) => ({
    id: `settings:${page.id}`,
    label: `Settings: ${page.label}`,
    section: SETTINGS_SECTION,
    detail: page.sections?.flatMap((s) => (s.title === page.label ? [] : [s.title])).join(", ") || undefined,
    run: () => runBusCommand(`app:settings:${page.id}`, "Could not open Settings"),
  }));
}

/** Everything the palette can run: the menus' items, then the page's own menus and the Settings pages. */
export function collectCommands(menu: MenuCommand[], page: PageSnapshot): PaletteCommand[] {
  const twins = new Map(allCommands().flatMap((c) => (c.bus ? [[c.bus, c] as const] : [])));
  return [...menu.flatMap((item) => fromMenu(item, page, twins) ?? []), ...page.own];
}
