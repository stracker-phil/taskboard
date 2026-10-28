import { Funnel } from "lucide-react";
import { useSidebar } from "../queries/sidebar";
import { narrowingKey, NO_NARROWING } from "../state/slices/narrowing";
import { useStore } from "../state/store";
import { findLevel } from "./levels";

/** A view with no issues (artboard 8, card 2): what the level asks for, and how to widen it. */
export function EmptyLevel({ listId, viewId }: { listId: number; viewId: number }) {
  const { data: tree } = useSidebar();
  const key = narrowingKey(listId, viewId);
  const narrowing = useStore((s) => s.narrowings[key]) ?? NO_NARROWING;
  const setNarrowing = useStore((s) => s.setNarrowing);
  const requestFilter = useStore((s) => s.requestFilter);
  const level = tree ? findLevel(tree, listId) : null;
  if (!level) return null;
  const name = level.kind === "space" ? level.space.name : level.node.name;
  const ownFilter = level.kind === "list" || (level.kind === "home" && level.node.kind === "list");
  const description = level.kind === "space" ? "" : level.node.description;
  const parts = [description, narrowing.me && "Assignee = me", narrowing.text.trim() && `the text “${narrowing.text.trim()}”`].filter(Boolean);
  const sentence = parts.length > 0 ? `No issues match ${parts.join(" and ")}.` : "No issues here yet.";
  return (
    <div className="empty-level">
      <span className="empty-level__icon" aria-hidden="true">
        <Funnel size={22} strokeWidth={1.8} />
      </span>
      <h2 className="empty-level__title">Nothing in {name}</h2>
      <p className="empty-level__text">{sentence}</p>
      {(ownFilter || narrowing.me) && (
        <div className="empty-level__actions">
          {ownFilter && (
            <button type="button" className="btn-secondary" onClick={requestFilter}>
              Edit filter
            </button>
          )}
          {narrowing.me && (
            <button type="button" className="empty-level__ghost" onClick={() => setNarrowing(key, { me: false })}>
              Clear Me filter
            </button>
          )}
        </div>
      )}
    </div>
  );
}
