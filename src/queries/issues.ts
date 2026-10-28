import { useQuery, type QueryClient } from "@tanstack/react-query";
import { api } from "../api";

/** Every issue's detail; `issueKey` is one issue's. */
export const ISSUE_KEY = ["issue"] as const;
export const issueKey = (id: string) => [...ISSUE_KEY, id] as const;

export const useIssue = (id: string | null) =>
  useQuery({ queryKey: issueKey(id ?? ""), queryFn: () => api.issueGet(id ?? ""), enabled: id !== null });

/** Loads an issue ahead, so moving to it shows it without a loading frame. */
export const prefetchIssue = (client: QueryClient, id: string) =>
  void client.prefetchQuery({ queryKey: issueKey(id), queryFn: () => api.issueGet(id) });
