import { useQuery, type QueryClient } from "@tanstack/react-query";
import { api } from "../api";

/** Every issue's detail; `issueKey` is one issue's. */
export const ISSUE_KEY = ["issue"] as const;
export const issueKey = (id: string) => [...ISSUE_KEY, id] as const;

export const useIssueActivity = (id: string) => useQuery({ queryKey: [...issueKey(id), "activity"], queryFn: () => api.issueActivity(id) });
export const useIssueRelated = (id: string) => useQuery({ queryKey: [...issueKey(id), "related"], queryFn: () => api.issueRelated(id) });
export const webLinksKey = (id: string) => [...issueKey(id), "webLinks"] as const;
export const useIssueWebLinks = (id: string) => useQuery({ queryKey: webLinksKey(id), queryFn: () => api.issueWebLinks(id) });

export const useIssue = (id: string | null) =>
  useQuery({ queryKey: issueKey(id ?? ""), queryFn: () => api.issueGet(id ?? ""), enabled: id !== null });

/** Loads an issue ahead, so moving to it shows it without a loading frame. */
export const prefetchIssue = (client: QueryClient, id: string) =>
  void client.prefetchQuery({ queryKey: issueKey(id), queryFn: () => api.issueGet(id) });
