import { useState } from "react";
import { Peek } from "../issue/Peek";
import { usePresence } from "../lib/motion";
import { useNav, useViews } from "../queries/sidebar";
import { narrowingKey } from "../state/slices/narrowing";
import { useStore } from "../state/store";

/** The current view's preview, when its place has one open; it slides out after closing. A Board opens issues
 *  in the full Issue view only. */
export function CurrentPeek() {
  const { data: nav } = useNav();
  const location = nav?.location;
  const { data: views } = useViews(location?.listId ?? null);
  // Until the views load, the layout is unknown; a Board must not flash a preview.
  const layout = views?.find((v) => v.id === location?.viewId)?.layout;
  const key = location ? narrowingKey(location.listId, location.viewId) : "";
  const peekId = useStore((s) => (s.places[key]?.peekOpen ? s.places[key]?.selectedId : null) ?? null);
  const updatePlace = useStore((s) => s.updatePlace);
  const openIssue = useStore((s) => s.openIssue);
  const shownId = location && layout && layout !== "board" ? peekId : null;
  // The issue it last showed stays while it slides out.
  const [lastId, setLastId] = useState<string | null>(null);
  if (shownId !== null && shownId !== lastId) setLastId(shownId);
  const { mounted, state } = usePresence(shownId !== null);
  if (!mounted || !location || !lastId) return null;
  const origin = { listId: location.listId, viewId: location.viewId };
  return (
    <div className={`peek-presence peek-presence--${state}`} inert={state === "leaving"}>
      <Peek id={lastId} onClose={() => updatePlace(origin.listId, origin.viewId, { peekOpen: false })} onExpand={() => openIssue(lastId, origin)} />
    </div>
  );
}
