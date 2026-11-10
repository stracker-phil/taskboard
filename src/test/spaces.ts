import type { FieldChoiceRow, FieldSetting, ListNode, SidebarTree, SpaceConfig, SpaceNode } from "../api/types";

const listNode = (patch: Partial<ListNode> = {}): ListNode => ({
  id: 1,
  kind: "everything",
  name: "Everything",
  count: null,
  ownConditions: 0,
  description: "",
  error: null,
  icon: null,
  color: null,
  ...patch,
});

/** A tracked Space as the sidebar tree holds it (the "Website relaunch" project); override per test. */
export function space(patch: Partial<SpaceNode> = {}): SpaceNode {
  return {
    listId: 11,
    projectId: "10001",
    key: "NW",
    name: "Website relaunch",
    projectName: "Website relaunch",
    icon: null,
    color: "1",
    accent: null,
    client: null,
    openCount: 21,
    lists: [],
    ...patch,
  };
}

/** A second Space (the "Lumen iOS app" project). */
export const lumenSpace = (patch: Partial<SpaceNode> = {}): SpaceNode =>
  space({ listId: 12, projectId: "10002", key: "LUM", name: "Lumen iOS app", projectName: "Lumen iOS app", ...patch });

/** A Space's configuration (the "Website relaunch" project): all layouts offered, grouped by status, one custom field. */
export function spaceConfig(patch: Partial<SpaceConfig> = {}): SpaceConfig {
  return {
    projectId: "10001",
    layouts: { layouts: ["list", "board", "table"], defaultLayout: "list", defaultGroup: "status" },
    fields: [fieldSetting()],
    ...patch,
  };
}

/** A custom field in a Space's config: the app shows it in the list and the Issue view, the Space too ("Reviewer"). */
export const fieldSetting = (patch: Partial<FieldSetting> = {}): FieldSetting => ({
  id: "customfield_10071",
  name: "Reviewer",
  appList: true,
  appDetail: true,
  inList: true,
  inDetail: true,
  ...patch,
});

/** A custom field as the app-wide Custom fields page lists it ("Reviewer", a user field, shown nowhere yet). */
export const fieldChoice = (patch: Partial<FieldChoiceRow> = {}): FieldChoiceRow => ({
  id: "customfield_10071",
  name: "Reviewer",
  kind: "user",
  inList: false,
  inDetail: false,
  ...patch,
});

/** A sidebar tree with the given Spaces in order. */
export function sidebarTree(spaces: SpaceNode[]): SidebarTree {
  return { everything: listNode(), home: [], pinnedLists: [], spaces };
}
