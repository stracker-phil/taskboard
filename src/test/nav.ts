import type { Nav, Tab } from "../api/types";

type Place = Nav["location"];

/** A tab at a place. */
export const tab = (id: number, location: Place): Tab => ({ id, location });

/** The nav of a window with one tab (id 1) at `location`; override per test. */
export const nav = (location: Place, patch: Partial<Nav> = {}): Nav => ({
  location,
  expanded: [],
  tabs: [{ id: 1, location }],
  activeTab: 1,
  ...patch,
});

/** The nav of a window with these tabs, `activeTab` showing; its location is the active tab's. */
export function tabbedNav(tabs: Tab[], activeTab: number, patch: Partial<Nav> = {}): Nav {
  return { location: tabs.find((t) => t.id === activeTab)?.location ?? null, expanded: [], tabs, activeTab, ...patch };
}
