import { Funnel, MessageSquare, X } from "lucide-react";
import { api, type PinItem } from "../api";
import { StatusSquare } from "../components/issue/StatusSquare";
import { usePins } from "../queries/pins";
import { useNavGo } from "../queries/sidebar";
import { attempt } from "../state/requests";
import { useStore } from "../state/store";

interface Props {
  listId: number;
  viewId: number;
}

/** Active pins, then the pins recently unpinned. */
export function PinnedView({ listId, viewId }: Props) {
  const { data } = usePins();
  const openIssue = useStore((s) => s.openIssue);
  const go = useNavGo();
  if (!data) return null;
  const open = (pin: PinItem) => {
    const target = pin.target;
    if (target.kind === "issue") openIssue(target.id, { listId, viewId });
    else if (target.kind === "comment") openIssue(target.issueId, { listId, viewId });
    else void go(target.id);
  };
  return (
    <div className="pinned">
      {data.active.length === 0 ? (
        <p className="pinned__empty">Nothing pinned. Press P on an issue, or pin a comment or a filter list.</p>
      ) : (
        <PinTable label="Bookmarked" pins={data.active} onOpen={open} onUnpin={(pin) => void attempt(api.pinRemove(pin.id), "Could not remove the bookmark")} />
      )}
      {data.recentlyUnpinned.length > 0 && (
        <section className="pinned__recent" aria-labelledby="pinned-recent">
          <h2 id="pinned-recent" className="pinned__caps">
            {RECENT}
          </h2>
          <PinTable label={RECENT} pins={data.recentlyUnpinned} onOpen={open} />
        </section>
      )}
    </div>
  );
}

const RECENT = "Recently removed";

interface TableProps {
  label: string;
  pins: PinItem[];
  onOpen: (pin: PinItem) => void;
  /** Active pins can be unpinned; ended ones are greyed and show no time left. */
  onUnpin?: (pin: PinItem) => void;
}

function PinTable({ label, pins, onOpen, onUnpin }: TableProps) {
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
          className="pin-table__row"
          role="row"
          tabIndex={0}
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
            {pin.timeLeft}
            {onUnpin && <UnpinButton onUnpin={() => onUnpin(pin)} />}
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

function Target({ pin }: { pin: PinItem }) {
  const target = pin.target;
  switch (target.kind) {
    case "issue":
      return (
        <>
          <StatusSquare style={target.style} />
          <span className="pin-table__key">{target.key}</span>
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
          <span className="pin-table__key">{target.issueKey}</span>
        </>
      );
    case "list":
      return (
        <>
          <Funnel className="pin-table__icon" size={14} strokeWidth={2} aria-hidden="true" />
          {target.space && <span className="pin-table__space">{target.space} /</span>}
          <span className="pin-table__text">{target.name}</span>
        </>
      );
  }
}
