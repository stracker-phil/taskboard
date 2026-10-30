import { LayoutTile } from "../shell/viewLayouts";
import { useQueryClient } from "@tanstack/react-query";
import { Folder, MessageSquare, X } from "lucide-react";
import { useEffect, useState } from "react";
import { CopyKey } from "../components/issue/CopyKey";
import { api, type PinItem } from "../api";
import { StatusSquare } from "../components/issue/StatusSquare";
import { PINS_KEY, usePins } from "../queries/pins";
import { useNavGo } from "../queries/sidebar";
import { attempt } from "../state/requests";
import { useStore } from "../state/store";

interface Props {
  listId: number;
  viewId: number;
}

/** Rust words the time left ("3h left"); asking again every minute keeps it counting down. */
const TIME_LEFT_REFRESH_MS = 60_000;

/** Active pins, then the pins recently unpinned. */
export function PinnedView({ listId, viewId }: Props) {
  const { data } = usePins();
  const client = useQueryClient();
  useEffect(() => {
    const timer = setInterval(() => void client.invalidateQueries({ queryKey: PINS_KEY }), TIME_LEFT_REFRESH_MS);
    return () => clearInterval(timer);
  }, [client]);
  const openIssue = useStore((s) => s.openIssue);
  const go = useNavGo();
  // Removed here, they keep their row (grayed, without the X) until the page is left, so the next X never slides
  // under the cursor (docs/ux.md → Stable controls).
  const [removed, setRemoved] = useState<Removed[]>([]);
  if (!data) return null;
  const removedIds = new Set(removed.map((r) => r.pin.id));
  const active = withRemoved(data.active.filter((pin) => !removedIds.has(pin.id)), removed);
  const unpin = (pin: PinItem) => {
    setRemoved((list) => [...list, { pin, at: active.findIndex((p) => p.id === pin.id) }]);
    void attempt(api.pinRemove(pin.id), "Could not remove the bookmark");
  };
  const open = (pin: PinItem) => {
    const target = pin.target;
    if (target.kind === "issue") openIssue(target.id, { listId, viewId });
    else if (target.kind === "comment") openIssue(target.issueId, { listId, viewId });
    else if (target.kind === "view") void go(target.listId, target.id);
    else void go(target.id);
  };
  return (
    <div className="pinned">
      {active.length === 0 ? (
        <p className="pinned__empty">Nothing pinned. Press P on an issue, or pin a comment or a filter list.</p>
      ) : (
        <PinTable label="Bookmarked" pins={active} onOpen={open} onUnpin={unpin} removed={removedIds} />
      )}
      {data.recentlyUnpinned.some((pin) => !removedIds.has(pin.id)) && (
        <section className="pinned__recent" aria-labelledby="pinned-recent">
          <h2 id="pinned-recent" className="pinned__caps">
            {RECENT}
          </h2>
          <PinTable label={RECENT} pins={data.recentlyUnpinned.filter((pin) => !removedIds.has(pin.id))} onOpen={open} />
        </section>
      )}
    </div>
  );
}

const RECENT = "Recently removed";

/** A pin removed on this page and the row it had. */
interface Removed {
  pin: PinItem;
  at: number;
}

/** `pins` with the ones removed here back at their rows. */
function withRemoved(pins: PinItem[], removed: Removed[]): PinItem[] {
  const rows = [...pins];
  for (const { pin, at } of [...removed].sort((a, b) => a.at - b.at)) rows.splice(Math.min(at, rows.length), 0, pin);
  return rows;
}

interface TableProps {
  label: string;
  pins: PinItem[];
  onOpen: (pin: PinItem) => void;
  /** Active pins can be unpinned; ended ones are grayed and show no time left. */
  onUnpin?: (pin: PinItem) => void;
  /** Pins just removed, still in their rows: grayed, no time left, no X. */
  removed?: Set<number>;
}

function PinTable({ label, pins, onOpen, onUnpin, removed }: TableProps) {
  return (
    <div className={onUnpin ? "pin-table" : "pin-table pin-table--ended"} role="table" aria-label={label}>
      <div className="pin-table__head" role="row">
        <span role="columnheader">Bookmarked</span>
        <span role="columnheader">Reason</span>
        <span role="columnheader">{onUnpin ? "Time left" : ""}</span>
      </div>
      {pins.map((pin) => (
        <div
          key={pin.id}
          className={removed?.has(pin.id) ? "pin-table__row pin-table__row--removed" : "pin-table__row"}
          role="row"
          tabIndex={0}
          data-issue-id={pin.target.kind === "issue" ? pin.target.id : undefined}
          onClick={() => onOpen(pin)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && e.target === e.currentTarget) onOpen(pin);
          }}
        >
          <span className="pin-table__name" role="cell">
            <Target pin={pin} />
          </span>
          <span className="pin-table__reason" role="cell">
            {pin.reason ?? ""}
          </span>
          <span className="pin-table__left" role="cell">
            {removed?.has(pin.id) ? "Removed" : pin.timeLeft}
            {onUnpin && (removed?.has(pin.id) ? <RemovedSlot /> : <UnpinButton onUnpin={() => onUnpin(pin)} />)}
          </span>
        </div>
      ))}
    </div>
  );
}

/** The row's own click opens the pin; this one must not. */
function UnpinButton({ onUnpin }: { onUnpin: () => void }) {
  return (
    <button
      type="button"
      className="pin-table__unpin"
      aria-label="Remove bookmark"
      title="Remove bookmark"
      onClick={(e) => {
        e.stopPropagation();
        onUnpin();
      }}
    >
      <X size={13} strokeWidth={2} aria-hidden="true" />
    </button>
  );
}

/** Where the X was: a second click there does nothing rather than opening the row. */
function RemovedSlot() {
  return <button type="button" className="pin-table__unpin" disabled tabIndex={-1} aria-hidden="true" onClick={(e) => e.stopPropagation()} />;
}

function Target({ pin }: { pin: PinItem }) {
  const target = pin.target;
  switch (target.kind) {
    case "issue":
      return (
        <>
          <StatusSquare style={target.style} />
          <CopyKey className="pin-table__key" issueKey={target.key} />
          <span className="pin-table__text">{target.summary}</span>
          {target.gone && <span className="pin-table__gone">gone</span>}
        </>
      );
    case "comment":
      return (
        <>
          <MessageSquare className="pin-table__icon" size={14} strokeWidth={1.9} aria-hidden="true" />
          <strong className="pin-table__author">{target.author?.name ?? "Someone"}</strong>
          <span className="pin-table__text">{target.excerpt}</span>
          <CopyKey className="pin-table__key" issueKey={target.issueKey} />
        </>
      );
    case "list":
      return (
        <>
          <Folder className="pin-table__icon" size={14} strokeWidth={2} aria-hidden="true" />
          {target.space && <span className="pin-table__space">{target.space} /</span>}
          <span className="pin-table__text">{target.name}</span>
        </>
      );
    case "view":
      return (
        <>
          <LayoutTile layout={target.layout} />
          <span className="pin-table__space">{target.space ? `${target.space} / ${target.list} ›` : `${target.list} ›`}</span>
          <span className="pin-table__text">{target.name}</span>
        </>
      );
  }
}
