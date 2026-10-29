import type { GroupBy, MenuEntry, Sort, SortBy } from "../../api";

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

/** The Group button's menu: the groupings, then Sort by ▸ with the sort keys and the direction. */
export function groupMenu(groupBy: GroupBy, sort: Sort, setGroup: (g: GroupBy) => void, setSort: (s: Sort) => void): MenuEntry[] {
  const groups = (Object.keys(GROUP_NAMES) as GroupBy[]).map((g) => ({ text: GROUP_NAMES[g], checked: g === groupBy, run: () => setGroup(g) }));
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
