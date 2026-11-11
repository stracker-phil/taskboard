import { keepPreviousData, useQuery } from "@tanstack/react-query";
import type { BoardData, Narrowing, View, ViewData } from "../api";
import { useViews } from "../queries/sidebar";
import { useRequestNarrowing, viewDataQuery } from "../queries/views";

const COUNT = new Intl.NumberFormat();

/** How many issues the view shows with its narrowing, at the view line's right end. Its slot stays empty until
 *  the view's first rows load. */
export function ViewCount({ listId, viewId }: { listId: number; viewId: number }) {
  const { data: views } = useViews(listId);
  const view = views?.find((v) => v.id === viewId);
  const narrowing = useRequestNarrowing(listId, viewId);
  return <span className="view-tabs__count">{view && narrowing && <Total view={view} narrowing={narrowing} />}</span>;
}

/** Reads the view's own query, so the number always matches the rows on screen. */
function Total({ view, narrowing }: { view: View; narrowing: Narrowing }) {
  const { data } = useQuery({ ...viewDataQuery(view, narrowing), placeholderData: keepPreviousData });
  const total = (data as ViewData | BoardData | undefined)?.total;
  if (total === undefined) return null;
  return <>{`${COUNT.format(total)} ${total === 1 ? "issue" : "issues"}`}</>;
}
