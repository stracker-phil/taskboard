import { CircleAlert } from "lucide-react";
import type { ListNode } from "../../api";

/** The open count, or a marker with the reason when the list's filter can't be used. */
export function ListCount({ list }: { list: ListNode }) {
  if (list.error) {
    return (
      <span className="sb-row__error" role="img" title={list.error} aria-label={list.error}>
        <CircleAlert size={13} strokeWidth={2} aria-hidden="true" />
      </span>
    );
  }
  return list.count === null ? null : <span className="sb-row__count">{list.count}</span>;
}
