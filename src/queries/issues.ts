import { keepPreviousData, useQuery, type QueryClient } from "@tanstack/react-query";
import { api } from "../api";

/** Every issue's detail; `issueKey` is one issue's. */
export const ISSUE_KEY = ["issue"] as const;
export const issueKey = (id: string) => [...ISSUE_KEY, id] as const;

export const useIssueActivity = (id: string) => useQuery({ queryKey: [...issueKey(id), "activity"], queryFn: () => api.issueActivity(id) });
export const useIssueRelated = (id: string) => useQuery({ queryKey: [...issueKey(id), "related"], queryFn: () => api.issueRelated(id) });
export const webLinksKey = (id: string) => [...issueKey(id), "webLinks"] as const;
export const useIssueWebLinks = (id: string) => useQuery({ queryKey: webLinksKey(id), queryFn: () => api.issueWebLinks(id) });

/** With `keepPrevious`, the last issue stays (as placeholder data) while the next one loads. */
export const useIssue = (id: string | null, { keepPrevious = false } = {}) =>
  useQuery({
    queryKey: issueKey(id ?? ""),
    queryFn: () => api.issueGet(id ?? ""),
    enabled: id !== null,
    placeholderData: keepPrevious ? keepPreviousData : undefined,
  });

/** Loads an issue ahead, so moving to it shows it without a loading frame. */
export const prefetchIssue = (client: QueryClient, id: string) =>
  void client.prefetchQuery({ queryKey: issueKey(id), queryFn: () => api.issueGet(id) });

/** Asks Rust to catch up on the issue's Jira history and web links, each at most every 5 minutes; a change comes
 *  back as `issue_details_changed`. Selecting a row starts it, so opening the issue finds them fresh. A failure
 *  (offline) keeps the cached ones. */
export function refreshIssueExtras(id: string) {
  void api.issueSyncHistory(id).catch(() => {});
  void api.issueSyncWebLinks(id).catch(() => {});
}
