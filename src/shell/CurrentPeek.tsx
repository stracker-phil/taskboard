import { Peek } from "../issue/Peek";
import { useNav, useViews } from "../queries/sidebar";
import { narrowingKey } from "../state/slices/narrowing";
import { useStore } from "../state/store";

/** The current view's peek, when its place has one open. A Board opens issues in the full Issue view only. */
export function CurrentPeek() {
  const { data: nav } = useNav();
  const location = nav?.location;
  const { data: views } = useViews(location?.listId ?? null);
  // Until the views load, the layout is unknown; a Board must not flash a peek.
  const layout = views?.find((v) => v.id === location?.viewId)?.layout;
  const key = location ? narrowingKey(location.listId, location.viewId) : "";
  const peekId = useStore((s) => (s.places[key]?.peekOpen ? s.places[key]?.selectedId : null) ?? null);
  const updatePlace = useStore((s) => s.updatePlace);
  const openIssue = useStore((s) => s.openIssue);
  if (!location || !peekId || !layout || layout === "board") return null;
  const origin = { listId: location.listId, viewId: location.viewId };
  return <Peek id={peekId} onClose={() => updatePlace(origin.listId, origin.viewId, { peekOpen: false })} onExpand={() => openIssue(peekId, origin)} />;
}
