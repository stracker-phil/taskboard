/** Settings pages by id and the search over their rows. Core and modules register their own; the Settings window
 *  reads them here. */
import { useSyncExternalStore, type ComponentType, type ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import { ranked } from "../lib/rank";

/** A group or row the search finds and `open_settings` scrolls to: `id` is its `anchor`, `title` its heading or
 *  label as shown. */
export interface SettingsRow {
  id: string;
  title: string;
  /** More words the search finds the row by. */
  keywords?: string[];
  /** The anchor to reveal while the row itself isn't shown, e.g. a switch only another setting adds. */
  under?: string;
}

export interface SettingsPage {
  id: string;
  group: string;
  label: string;
  icon: LucideIcon;
  intro?: ReactNode;
  Body: ComponentType;
  /** Position inside its group; pages without one follow, by label. */
  order?: number;
  /** More words the search finds the page by. */
  keywords?: string[];
  rows?: SettingsRow[];
}

/** Nav order of the known groups; any other group follows them, by name. */
const GROUP_ORDER = ["Jira", "Workspace", "System"];

const pages = new Map<string, SettingsPage>();
const listeners = new Set<() => void>();
let version = 0;

/** Adds a page, or replaces the one with the same id. */
export function registerSettingsPage(page: SettingsPage): void {
  pages.set(page.id, page);
  version += 1;
  listeners.forEach((notify) => notify());
}

function groupRank(group: string): number {
  const index = GROUP_ORDER.indexOf(group);
  return index === -1 ? GROUP_ORDER.length : index;
}

function compare(a: SettingsPage, b: SettingsPage): number {
  const byGroup = groupRank(a.group) - groupRank(b.group) || a.group.localeCompare(b.group);
  if (byGroup !== 0) return byGroup;
  const rank = (p: SettingsPage) => p.order ?? Number.MAX_SAFE_INTEGER;
  const byOrder = rank(a) - rank(b);
  if (byOrder !== 0) return byOrder;
  return a.label.localeCompare(b.label);
}

/** Every page in nav order: by group, then `order`, then label. */
export function settingsPages(): SettingsPage[] {
  return [...pages.values()].sort(compare);
}

/** `space:<projectId>` opens one Space's settings on the Spaces page. */
export const SPACE_TARGET = "space:";

export const findRow = (page: SettingsPage, id: string | undefined) => page.rows?.find((row) => row.id === id);

const isSpaceTarget = (page: SettingsPage, id: string | undefined) => page.id === "spaces" && !!id?.startsWith(SPACE_TARGET);

/** The named page, the page holding the named row (or Space), or the first when the name is absent or unknown. */
export function resolveSettingsPage(id: string | undefined): SettingsPage | undefined {
  const all = settingsPages();
  return all.find((p) => p.id === id) ?? all.find((p) => findRow(p, id) || isSpaceTarget(p, id)) ?? all[0];
}

/** What a page opens scrolled to for `id`: one of its rows, or a Space on the Spaces page. */
export function targetOn(page: SettingsPage, id: string | undefined): { id: string; under?: string } | undefined {
  const row = findRow(page, id);
  if (row) return { id: row.id, under: row.under };
  return id && isSpaceTarget(page, id) ? { id } : undefined;
}

/** A search hit; `row` is absent when the page itself matched. */
export interface SettingsMatch {
  page: SettingsPage;
  row?: SettingsRow;
}

/** Every page and row matching `query`, best first, ranked like the palette. */
export function searchSettings(query: string): SettingsMatch[] {
  const entries = settingsPages().flatMap((page): SettingsMatch[] => [{ page }, ...(page.rows ?? []).map((row) => ({ page, row }))]);
  return ranked(entries, query, ({ page, row }) =>
    row ? [row.title, (row.keywords ?? []).join(" ")] : [page.label, [page.group, ...(page.keywords ?? [])].join(" ")],
  );
}

/** The pages in nav order, in runs that share a group. */
export function settingsGroups(): { group: string; pages: SettingsPage[] }[] {
  const groups: { group: string; pages: SettingsPage[] }[] = [];
  for (const page of settingsPages()) {
    const last = groups.at(-1);
    if (last?.group === page.group) last.pages.push(page);
    else groups.push({ group: page.group, pages: [page] });
  }
  return groups;
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => void listeners.delete(listener);
}

/** Re-renders the caller when a page is registered, so a module may register after the window opened. */
export function useSettingsPagesVersion(): number {
  return useSyncExternalStore(subscribe, () => version);
}
