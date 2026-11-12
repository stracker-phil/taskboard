import { keepPreviousData, skipToken, useQuery } from "@tanstack/react-query";
import type { BoardData, ViewData } from "../api";
import { useViews } from "../queries/sidebar";
import { useRequestNarrowing, viewDataQuery } from "../queries/views";

const COUNT = new Intl.NumberFormat();

/** How many rows the view shows with its narrowing, read from the view's own query so it always matches them;
 *  none until its first rows load. */
export function useViewTotal(listId: number, viewId: number): number | undefined {
  const { data: views } = useViews(listId);
  const view = views?.find((v) => v.id === viewId);
  const narrowing = useRequestNarrowing(listId, viewId);
  const query = view && narrowing ? viewDataQuery(view, narrowing) : undefined;
  const { data } = useQuery({
    queryKey: query?.queryKey ?? ["viewTotal"],
    queryFn: query?.queryFn ?? skipToken,
    gcTime: query?.gcTime,
    meta: query?.meta,
    placeholderData: keepPreviousData,
  });
  return (data as ViewData | BoardData | undefined)?.total;
}

export const issueCount = (n: number) => `${COUNT.format(n)} ${n === 1 ? "issue" : "issues"}`;
export const matchCount = (n: number) => `${COUNT.format(n)} ${n === 1 ? "issue matches" : "issues match"}`;
