import { CircleAlert } from "lucide-react";
import type { ListNode } from "../../api";
import { useModes } from "../../queries/modes";

/** The open count, or with Unread only the unread count after a dot (none at 0); a marker with the reason when the
 *  list's filter can't be used. Bookmarks always counts its bookmarks. */
export function ListCount({ list }: { list: ListNode }) {
  const { unreadOnly } = useModes();
  if (list.error) {
    return (
      <span className="sb-row__error" role="img" title={list.error} aria-label={list.error}>
        <CircleAlert size={13} strokeWidth={2} aria-hidden="true" />
      </span>
    );
  }
  if (list.count === null) return null;
  if (!unreadOnly || list.kind === "pinned") return <span className="sb-row__count">{list.count}</span>;
  if (list.count === 0) return null;
  return (
    <span className="sb-row__count sb-row__count--unread" aria-label={`${list.count} unread`}>
      <span className="sb-row__unread-dot" aria-hidden="true" />
      {list.count}
    </span>
  );
}
