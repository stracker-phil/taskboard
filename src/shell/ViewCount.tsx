import { issueCount, useViewTotal } from "./viewTotal";

/** The view's row count at the view line's right end. */
export function ViewCount({ listId, viewId }: { listId: number; viewId: number }) {
  const total = useViewTotal(listId, viewId);
  return <span className="view-tabs__count">{total !== undefined && issueCount(total)}</span>;
}
