import { Peek } from "../issue/Peek";
import { useNav } from "../queries/sidebar";
import { narrowingKey } from "../state/slices/narrowing";
import { useStore } from "../state/store";

/** The current view's peek, when its place has one open. */
export function CurrentPeek() {
  const { data: nav } = useNav();
  const location = nav?.location;
  const key = location ? narrowingKey(location.listId, location.viewId) : "";
  const peekId = useStore((s) => (s.places[key]?.peekOpen ? s.places[key]?.selectedId : null) ?? null);
  const updatePlace = useStore((s) => s.updatePlace);
  const openIssue = useStore((s) => s.openIssue);
  if (!location || !peekId) return null;
  const origin = { listId: location.listId, viewId: location.viewId };
  return <Peek id={peekId} onClose={() => updatePlace(origin.listId, origin.viewId, { peekOpen: false })} onExpand={() => openIssue(peekId, origin)} />;
}
