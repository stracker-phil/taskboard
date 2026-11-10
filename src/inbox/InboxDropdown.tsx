import { useEffect, useRef, type RefObject } from "react";
import { api } from "../api";
import { holdUnread } from "../issue/useSeenAfter";
import { useNav } from "../queries/sidebar";
import { useOpenIssueInTab } from "../shell/openInTab";
import { attempt } from "../state/requests";
import { useStore } from "../state/store";
import { InboxPreview } from "./InboxPreview";
import { InboxRow } from "./InboxRow";
import { onInboxKeys } from "./inboxKeys";
import { useInboxState } from "./inboxState";
import { useInboxRows } from "./queries";
import { useDismiss } from "./useDismiss";

interface Props {
  bell: RefObject<HTMLButtonElement | null>;
  onClose: () => void;
}

/** Every issue that was unread when it opened, newest change first. Rows read since stay, dimmed, in place until
 *  it closes. Not a menu: the list's keys move through it (`inboxKeys`). */
export function InboxDropdown({ bell, onClose }: Props) {
  const { rows, unread } = useInboxRows();
  const { selected, select, previewing, setPreviewing } = useInboxState();
  const ref = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLUListElement>(null);
  const { data: nav } = useNav();
  const openInTab = useOpenIssueInTab();
  const location = nav?.location ?? null;

  const index = Math.min(selected, rows.length - 1);
  const current = rows[index];
  const toggleRead = (id: string, unreadNow: boolean) => {
    // Marked unread while previewed, it stays unread until previewed again.
    if (!unreadNow) holdUnread(id);
    void attempt(unreadNow ? api.issuesMarkSeen([id]) : api.issueMarkUnread(id), "Could not change the read state");
  };
  const markAllRead = () => void attempt(api.inboxMarkAllSeen(), "Could not mark the notifications as read");
  const open = (at: number, inNewTab: boolean) => {
    const row = rows[at];
    if (!row || !location) return;
    const origin = { listId: location.listId, viewId: location.viewId };
    onClose();
    if (inNewTab) void openInTab(row.id, origin, true);
    else useStore.getState().openIssue(row.id, origin);
  };

  useInboxKeys({
    move: (step) => {
      if (rows.length > 0) select(Math.max(0, Math.min(rows.length - 1, selected + step)));
    },
    toggleRead: () => current && toggleRead(current.id, current.unread),
    open: (inNewTab) => open(index, inNewTab),
    markAllRead,
    togglePreview: () => setPreviewing(!previewing),
    close: () => (previewing ? setPreviewing(false) : onClose()),
  });
  useDismiss(ref, bell, onClose, ".hover-peek--interactive");

  useEffect(() => {
    listRef.current?.querySelector(".inbox__row--selected")?.scrollIntoView?.({ block: "nearest" });
  }, [selected]);

  return (
    <div ref={ref} className="inbox popover" role="dialog" aria-label="Notifications" tabIndex={-1}>
      <div className="popover__header">
        <span className="popover__title">Notifications</span>
        <span className="inbox__count">{unread === 0 ? "" : `${unread} unread`}</span>
        <button type="button" className="inbox__mark-all" disabled={unread === 0} onClick={markAllRead}>
          Mark all as read
        </button>
      </div>
      {rows.length === 0 ? (
        <p className="inbox__empty">You're all caught up.</p>
      ) : (
        <ul ref={listRef} className="inbox__rows" aria-label="Unread issues">
          {rows.map((row, at) => (
            <InboxRow key={row.id} row={row} selected={at === index} onHover={() => select(at)} onOpen={(inNewTab) => open(at, inNewTab)} />
          ))}
        </ul>
      )}
      {previewing && current && <InboxPreview key={current.id} id={current.id} unread={current.unread} beside={ref} />}
    </div>
  );
}

/** Hands the inbox's keys the latest handlers while it is open. */
function useInboxKeys(handlers: Parameters<typeof onInboxKeys>[0]) {
  const latest = useRef(handlers);
  useEffect(() => {
    latest.current = handlers;
  });
  useEffect(
    () =>
      onInboxKeys({
        move: (step) => latest.current.move(step),
        toggleRead: () => latest.current.toggleRead(),
        open: (inNewTab) => latest.current.open(inNewTab),
        markAllRead: () => latest.current.markAllRead(),
        togglePreview: () => latest.current.togglePreview(),
        close: () => latest.current.close(),
      }),
    [],
  );
}
