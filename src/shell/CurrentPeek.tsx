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
  if (!location || !peekId) return null;
  return <Peek id={peekId} onClose={() => updatePlace(location.listId, location.viewId, { peekOpen: false })} />;
}
