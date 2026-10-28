import type { ListNode, View } from "../api/types";

/** A view tab of the "Website relaunch" Space (list 11); override per test. */
export function view(patch: Partial<View> = {}): View {
  return {
    id: 101,
    listId: 11,
    name: "List",
    layout: "list",
    columns: [],
    groupBy: "status",
    sort: { by: "updated", descending: true },
    widths: {},
    wrapTitles: true,
    collapsed: {},
    position: 0,
    isDefault: false,
    ...patch,
  };
}

/** A filter list or Home list as the sidebar tree holds it; override per test. */
export function listNode(patch: Partial<ListNode> = {}): ListNode {
  return { id: 21, kind: "list", name: "My open issues", count: 4, ownConditions: 1, description: "", error: null, icon: null, ...patch };
}
