import type { ListNode, SidebarTree, SpaceNode } from "../api/types";

const listNode = (patch: Partial<ListNode> = {}): ListNode => ({
  id: 1,
  kind: "everything",
  name: "Everything",
  count: null,
  ownConditions: 0,
  description: "",
  error: null,
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
    client: null,
    openCount: 21,
    lists: [],
    ...patch,
  };
}

/** A second Space (the "Lumen iOS app" project). */
export const lumenSpace = (patch: Partial<SpaceNode> = {}): SpaceNode =>
  space({ listId: 12, projectId: "10002", key: "LUM", name: "Lumen iOS app", projectName: "Lumen iOS app", ...patch });

/** A sidebar tree with the given Spaces in order. */
export function sidebarTree(spaces: SpaceNode[]): SidebarTree {
  return { everything: listNode(), home: [], spaces };
}
