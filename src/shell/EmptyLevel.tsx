import { Funnel } from "lucide-react";
import { api } from "../api";
import { useModes } from "../queries/modes";
import { useSidebar } from "../queries/sidebar";
import { NO_NARROWING, useRequestNarrowing } from "../queries/views";
import { attempt } from "../state/requests";
import { narrowingKey } from "../state/slices/narrowing";
import { useStore } from "../state/store";
import { findLevel } from "./levels";
import { useMeMode } from "./toolbar/useMeMode";
import { useEmptyReason } from "./useEmptyReason";

/** A view with no issues (artboard 8, card 2): why it's empty in a few words, and one button per way to widen it. */
export function EmptyLevel({ listId, viewId }: { listId: number; viewId: number }) {
  const { data: tree } = useSidebar();
  const key = narrowingKey(listId, viewId);
  const narrowing = useRequestNarrowing(listId, viewId) ?? NO_NARROWING;
  const { unreadOnly } = useModes();
  const me = useMeMode(key, narrowing.me, narrowing.person, narrowing.meOn);
  const { suffix, viewFiltered } = useEmptyReason(listId, viewId, narrowing, me, unreadOnly);
  const setNarrowing = useStore((s) => s.setNarrowing);
  const requestFilter = useStore((s) => s.requestFilter);
  const level = tree ? findLevel(tree, listId) : null;
  if (!level) return null;
  const name = level.kind === "space" ? level.space.name : level.node.name;
  const listFiltered = level.kind === "list" || (level.kind === "home" && level.node.kind === "list");
  const editsFilter = listFiltered || viewFiltered;
  const text = narrowing.text.trim();
  const narrowed = editsFilter || me.on || me.person !== null || text !== "" || narrowing.epic !== null || unreadOnly;
  const widen = [
    text && { label: "Clear search", run: () => setNarrowing(key, { text: "" }) },
    me.on && { label: "Clear Me Mode", run: () => me.setMe(false) },
    me.person && { label: "Show anyone", run: () => me.setPerson(null) },
    unreadOnly && { label: "Turn off Unread only", run: () => void attempt(api.modeSet("unreadOnly", false), "Could not change Unread only") },
  ].filter((action): action is { label: string; run: () => void } => Boolean(action));
  return (
    <div className="empty-level">
      <span className="empty-level__icon" aria-hidden="true">
        <Funnel size={22} strokeWidth={1.8} />
      </span>
      {/* Named as one sentence: the reason's leading space is lost when the span's text is joined. */}
      <h2 className="empty-level__title" aria-label={`Nothing in ${name}${suffix ?? ""}`}>
        Nothing in {name}
        {/* Keyed, so a changed reason fades in again. */}
        {suffix && (
          <span key={suffix} className="empty-level__reason">
            {suffix}
          </span>
        )}
      </h2>
      <p className="empty-level__text">{narrowed ? "Looks like everything is done here." : "No issues here yet."}</p>
      {(editsFilter || widen.length > 0) && (
        <div className="empty-level__actions">
          {editsFilter && (
            <button type="button" className="btn-secondary" onClick={requestFilter}>
              Edit filter
            </button>
          )}
          {widen.map((action) => (
            <button key={action.label} type="button" className="empty-level__ghost" onClick={action.run}>
              {action.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
