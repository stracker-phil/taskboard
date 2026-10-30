import { QueryCache, QueryClient } from "@tanstack/react-query";
import { fail } from "../state/requests";

declare module "@tanstack/react-query" {
  interface Register {
    /** `failure` names what could not load; a failed query with it shows as an error toast. */
    queryMeta: { failure?: string };
  }
}

/** Data changes only when Rust says so (an event, see `useEvents`): no retries, never stale, no refetch on focus. */
export function createQueryClient(): QueryClient {
  return new QueryClient({
    queryCache: new QueryCache({
      onError: (error, query) => {
        if (query.meta?.failure) fail(error, query.meta.failure);
      },
    }),
    defaultOptions: {
      queries: { retry: false, staleTime: Infinity, refetchOnWindowFocus: false },
      mutations: { retry: false },
    },
  });
}

/** The page's one client; tests make a fresh one with `createQueryClient`. */
export const queryClient = createQueryClient();
