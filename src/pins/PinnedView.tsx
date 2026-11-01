import { LayoutTile } from "../shell/viewLayouts";
import { useQueryClient } from "@tanstack/react-query";
import { Folder, MessageSquare, X } from "lucide-react";
import { useEffect, useState, type HTMLAttributes, type MouseEvent, type ReactNode, type RefObject } from "react";
import { CopyKey } from "../components/issue/CopyKey";
import { api, type PinItem } from "../api";
import { InlineRename } from "../components/InlineRename";
import { StatusSquare } from "../components/issue/StatusSquare";
import { openContextMenu } from "../components/menu";
import { PINS_KEY, usePins } from "../queries/pins";
import { DropLine } from "../shell/Sidebar/DropLine";
import { useDragReorder } from "../shell/Sidebar/useDragReorder";
import { attempt } from "../state/requests";
import { pinName, useOpenPin } from "./openPin";

/** Rust words the time left ("3h left"); asking again every minute keeps it counting down. */
const TIME_LEFT_REFRESH_MS = 60_000;

/** Active pins in the owner's order (drag a row to move it), then the pins recently unpinned. */
export function PinnedView() {
  const { data } = usePins();
  const client = useQueryClient();
  useEffect(() => {
    const timer = setInterval(() => void client.invalidateQueries({ queryKey: PINS_KEY }), TIME_LEFT_REFRESH_MS);
    return () => clearInterval(timer);
  }, [client]);
  const open = useOpenPin();
  const [renaming, setRenaming] = useState<number | null>(null);
  const activePins = data?.active ?? [];
  const { containerRef, item, handle, dropLineY } = useDragReorder<HTMLDivElement>((from, to) => {
    const moved = activePins[from];
    if (!moved) return;
    const rest = activePins.filter((pin) => pin.id !== moved.id);
    void attempt(api.pinMove(moved.id, rest[to]?.id ?? null), "Could not move the bookmark");
  });
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
  const rename = (pin: PinItem, title: string | null) => void attempt(api.pinRename(pin.id, title), "Could not rename the bookmark");
  const menu = (e: MouseEvent, pin: PinItem) =>
    openContextMenu(
      e,
      [
        { text: "Open", run: () => open(pin) },
        { text: "Rename Bookmark…", run: () => setRenaming(pin.id) },
        ...(pin.title ? [{ text: "Use Original Name", hint: pinName({ ...pin, title: null }), run: () => rename(pin, null) }] : []),
        "separator",
        { text: "Remove Bookmark", run: () => unpin(pin) },
      ],
      pinName(pin),
    );
  // Removed rows stay where they were and can't be dragged: the order is the active pins'.
  const reorder = {
    containerRef,
    dropLineY,
    rowProps: (pin: PinItem): RowProps => {
      const index = activePins.findIndex((p) => p.id === pin.id);
      if (index === -1 || removedIds.has(pin.id)) return {};
      return pin.id === renaming ? item(index) : { ...item(index), ...handle(index) };
    },
  };
  return (
    <div className="pinned">
      {active.length === 0 ? (
        <p className="pinned__empty">Nothing pinned. Press P on an issue, or pin a comment or a filter list.</p>
      ) : (
        <PinTable
          label="Bookmarked"
          pins={active}
          onOpen={open}
          onUnpin={unpin}
          removed={removedIds}
          onMenu={menu}
          reorder={reorder}
          renaming={renaming}
          onRename={(pin, title) => rename(pin, title)}
          onRenameDone={() => setRenaming(null)}
        />
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

/** What a row takes to be dragged. */
type RowProps = HTMLAttributes<HTMLElement> & { "data-drag-index"?: number; "data-dragging"?: boolean };

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
  onMenu?: (e: MouseEvent, pin: PinItem) => void;
  reorder?: {
    containerRef: RefObject<HTMLDivElement | null>;
    dropLineY: number | null;
    rowProps: (pin: PinItem) => RowProps;
  };
  /** The bookmark whose name is edited in place. */
  renaming?: number | null;
  onRename?: (pin: PinItem, title: string) => void;
  onRenameDone?: () => void;
}

function PinTable({ label, pins, onOpen, onUnpin, removed, onMenu, reorder, renaming, onRename, onRenameDone }: TableProps) {
  return (
    <div ref={reorder?.containerRef} className={onUnpin ? "pin-table" : "pin-table pin-table--ended"} role="table" aria-label={label}>
      <div className="pin-table__head" role="row">
        <span role="columnheader">Bookmarked</span>
        <span role="columnheader">Reason</span>
        <span role="columnheader">{onUnpin ? "Time left" : ""}</span>
      </div>
      {pins.map((pin) => (
        <div
          key={pin.id}
          {...reorder?.rowProps(pin)}
          className={removed?.has(pin.id) ? "pin-table__row pin-table__row--removed" : "pin-table__row"}
          role="row"
          tabIndex={0}
          data-issue-id={pin.target.kind === "issue" ? pin.target.id : undefined}
          onClick={() => onOpen(pin)}
          onContextMenu={onMenu ? (e) => onMenu(e, pin) : undefined}
          onKeyDown={(e) => {
            if (e.key === "Enter" && e.target === e.currentTarget) onOpen(pin);
          }}
        >
          <span className="pin-table__name" role="cell">
            <Target pin={pin}>
              {pin.id === renaming && onRename && onRenameDone ? (
                <InlineRename value={pinName(pin)} label="Bookmark name" onSave={(title) => onRename(pin, title)} onDone={onRenameDone} className="pin-table__rename" />
              ) : (
                <span className="pin-table__text" title={pin.title ? `${pinName({ ...pin, title: null })}` : undefined}>
                  {pinName(pin)}
                </span>
              )}
            </Target>
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
      {reorder && <DropLine y={reorder.dropLineY} />}
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

/** The item's marks around its name (`children`): status and key, author, Space and list. */
function Target({ pin, children }: { pin: PinItem; children: ReactNode }) {
  const target = pin.target;
  switch (target.kind) {
    case "issue":
      return (
        <>
          <StatusSquare style={target.style} />
          <CopyKey className="pin-table__key" issueKey={target.key} />
          {children}
          {target.gone && <span className="pin-table__gone">gone</span>}
        </>
      );
    case "comment":
      return (
        <>
          <MessageSquare className="pin-table__icon" size={14} strokeWidth={1.9} aria-hidden="true" />
          <strong className="pin-table__author">{target.author?.name ?? "Someone"}</strong>
          {children}
          <CopyKey className="pin-table__key" issueKey={target.issueKey} />
        </>
      );
    case "list":
      return (
        <>
          <Folder className="pin-table__icon" size={14} strokeWidth={2} aria-hidden="true" />
          {target.space && <span className="pin-table__space">{target.space} /</span>}
          {children}
        </>
      );
    case "view":
      return (
        <>
          <LayoutTile layout={target.layout} />
          <span className="pin-table__space">{target.space ? `${target.space} / ${target.list} ›` : `${target.list} ›`}</span>
          {children}
        </>
      );
  }
}
