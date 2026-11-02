import type { GroupBy, Sort, SortBy } from "../../api";
import type { MenuEntry } from "../../components/menu";

export const GROUP_NAMES: Record<GroupBy, string> = {
  status: "Status",
  statusType: "Status type",
  project: "Space",
  client: "Client",
  assignee: "Assignee",
  epic: "Epic",
  release: "Release",
  priority: "Priority",
  due: "Due date",
  none: "None",
};

const SORT_NAMES: Record<SortBy, string> = {
  updated: "Updated",
  created: "Created",
  priority: "Priority",
  key: "Key",
  due: "Due date",
  summary: "Summary",
  status: "Status",
};

/** Groupings that only tell issues apart across Spaces. */
const ACROSS_SPACES: GroupBy[] = ["project", "client"];

/** The Group button's menu: the groupings, then Sort by ▸ with the sort keys and the direction. Space and Client
 *  are offered only where issues of several Spaces meet (`acrossSpaces`), or while the view already uses one. */
export function groupMenu(groupBy: GroupBy, sort: Sort, acrossSpaces: boolean, setGroup: (g: GroupBy) => void, setSort: (s: Sort) => void): MenuEntry[] {
  const offered = (Object.keys(GROUP_NAMES) as GroupBy[]).filter((g) => acrossSpaces || g === groupBy || !ACROSS_SPACES.includes(g));
  const groups = offered.map((g) => ({ text: GROUP_NAMES[g], checked: g === groupBy, run: () => setGroup(g) }));
  const keys = (Object.keys(SORT_NAMES) as SortBy[]).map((by) => ({ text: SORT_NAMES[by], checked: by === sort.by, run: () => setSort({ ...sort, by }) }));
  return [
    ...groups,
    "separator",
    {
      text: "Sort by",
      items: [
        ...keys,
        "separator",
        { text: "Ascending", checked: !sort.descending, run: () => setSort({ ...sort, descending: false }) },
        { text: "Descending", checked: sort.descending, run: () => setSort({ ...sort, descending: true }) },
      ],
    },
  ];
}
