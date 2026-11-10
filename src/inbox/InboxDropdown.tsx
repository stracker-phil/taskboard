import { useEffect, useRef, type RefObject } from "react";
import { api } from "../api";
import { StatusSquare } from "../components/issue/StatusSquare";
import { timeAgo } from "../lib/relativeTime";
import { useNav } from "../queries/sidebar";
import { useOpenIssueInTab } from "../shell/openInTab";
import { attempt } from "../state/requests";
import { useStore } from "../state/store";
import { onInboxKeys } from "./inboxKeys";
import { useInboxState } from "./inboxState";
import { useInboxRows } from "./queries";

interface Props {
  bell: RefObject<HTMLButtonElement | null>;
  onClose: () => void;
}

/** Every issue that was unread when it opened, newest change first. Rows read since stay, dimmed, in place until
 *  it closes. Not a menu: the list's keys move through it (`inboxKeys`). */
export function InboxDropdown({ bell, onClose }: Props) {
  const { rows, unread } = useInboxRows();
  const selected = useInboxState((s) => s.selected);
  const select = useInboxState((s) => s.select);
  const ref = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLUListElement>(null);
  const { data: nav } = useNav();
  const openInTab = useOpenIssueInTab();
  const location = nav?.location ?? null;

  const index = Math.min(selected, rows.length - 1);
  const current = rows[index];
  const toggleRead = (id: string, unreadNow: boolean) =>
    void attempt(unreadNow ? api.issuesMarkSeen([id]) : api.issueMarkUnread(id), "Could not change the read state");
  const markAllRead = () => void attempt(api.inboxMarkAllSeen(), "Could not mark the notifications as read");
  const open = (at: number, inNewTab: boolean) => {
    const row = rows[at];
    if (!row || !location) return;
    const origin = { listId: location.listId, viewId: location.viewId };
    onClose();
    if (inNewTab) void openInTab(row.id, origin, true);
    else useStore.getState().openIssue(row.id, origin);
  };

  const latest = useRef({ rows, selected, index, current, open, toggleRead, markAllRead, onClose });
  useEffect(() => {
    latest.current = { rows, selected, index, current, open, toggleRead, markAllRead, onClose };
  });
  useEffect(
    () =>
      onInboxKeys({
        move: (step) => {
          const last = latest.current.rows.length - 1;
          if (last >= 0) select(Math.max(0, Math.min(last, latest.current.selected + step)));
        },
        toggleRead: () => {
          const row = latest.current.current;
          if (row) latest.current.toggleRead(row.id, row.unread);
        },
        open: (inNewTab) => latest.current.open(latest.current.index, inNewTab),
        markAllRead: () => latest.current.markAllRead(),
        close: () => latest.current.onClose(),
      }),
    [select],
  );

  useEffect(() => {
    const panel = ref.current;
    panel?.focus();
    const opener = bell.current;
    return () => {
      const active = document.activeElement;
      if (!active || active === document.body || panel?.contains(active)) opener?.focus();
    };
  }, [bell]);

  useEffect(() => {
    const onPress = (e: MouseEvent) => {
      const target = e.target as Node;
      if (ref.current?.contains(target) || bell.current?.contains(target)) return;
      latest.current.onClose();
    };
    window.addEventListener("mousedown", onPress, true);
    return () => window.removeEventListener("mousedown", onPress, true);
  }, [bell]);

  useEffect(() => {
    listRef.current?.querySelector(".inbox__row--selected")?.scrollIntoView?.({ block: "nearest" });
  }, [selected]);

  return (
    <div ref={ref} className="inbox popover" role="dialog" aria-label="Notifications" tabIndex={-1}>
      <div className="popover__header">
        <span className="popover__title">Notifications</span>
        <span className="inbox__count">{unread === 0 ? "" : `${unread} unread`}</span>
        <button
          type="button"
          className="inbox__mark-all"
          disabled={unread === 0}
          onClick={markAllRead}
        >
          Mark all as read
        </button>
      </div>
      {rows.length === 0 ? (
        <p className="inbox__empty">You're all caught up.</p>
      ) : (
        <ul ref={listRef} className="inbox__rows" aria-label="Unread issues">
          {rows.map((row, at) => (
            <li
              key={row.id}
              className={["inbox__row", at === index && "inbox__row--selected", !row.unread && "inbox__row--read"].filter(Boolean).join(" ")}
              aria-current={at === index ? "true" : undefined}
              onMouseEnter={() => select(at)}
              onClick={(e) => open(at, e.metaKey)}
            >
              <span className="inbox__line">
                <StatusSquare style={row.style} />
                <span className="inbox__key">{row.key}</span>
                <span className="inbox__summary">{row.summary}</span>
              </span>
              <span className="inbox__note">
                <span className="inbox__note-text">{row.note}</span>
                <span className="inbox__time">{timeAgo(row.changedAt)}</span>
              </span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
