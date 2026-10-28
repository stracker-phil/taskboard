import { QueryClient } from "@tanstack/react-query";

/** Data changes only when Rust says so (an event, see `useEvents`): no retries, never stale, no refetch on focus. */
export function createQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, staleTime: Infinity, refetchOnWindowFocus: false },
      mutations: { retry: false },
    },
  });
}

/** The page's one client; tests make a fresh one with `createQueryClient`. */
export const queryClient = createQueryClient();
