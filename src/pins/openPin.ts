import type { PinItem } from "../api";
import { useNav, useNavGo } from "../queries/sidebar";
import type { IssueOrigin } from "../state/slices/issueView";
import { useStore } from "../state/store";

/** Opens what a bookmark points at: an issue or a comment's issue over the current view, a view at its level, a
 *  list or Space. */
export function useOpenPin() {
  const openIssue = useStore((s) => s.openIssue);
  const go = useNavGo();
  const { data: nav } = useNav();
  const origin: IssueOrigin | null = nav?.location ? { listId: nav.location.listId, viewId: nav.location.viewId } : null;
  const showIssue = (id: string) => (origin ? openIssue(id, origin) : undefined);
  return (pin: PinItem) => {
    const target = pin.target;
    if (target.kind === "issue") showIssue(target.id);
    else if (target.kind === "comment") showIssue(target.issueId);
    else if (target.kind === "view") void go(target.listId, target.id);
    else void go(target.id);
  };
}

/** The name a bookmark shows: the owner's own, else the item's. */
export function pinName(pin: PinItem): string {
  if (pin.title) return pin.title;
  const target = pin.target;
  switch (target.kind) {
    case "issue":
      return target.summary;
    case "comment":
      return target.excerpt;
    case "list":
    case "view":
      return target.name;
  }
}

/** The level and view a list or view bookmark opens; none for an issue or a comment. */
export function pinPlace(pin: PinItem): { listId: number; viewId: number | null } | null {
  const target = pin.target;
  if (target.kind === "view") return { listId: target.listId, viewId: target.id };
  if (target.kind === "list") return { listId: target.id, viewId: null };
  return null;
}
