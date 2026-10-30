import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

/** macOS's default, until the system's own interval has loaded. */
const DEFAULT_MS = 500;

/** The system's double-click interval in milliseconds; read again on focus, as it may change in System Settings. */
export function useDoubleClickMs(): number {
  const { data } = useQuery({ queryKey: ["doubleClickMs"], queryFn: api.doubleClickMs, staleTime: 0, refetchOnWindowFocus: true });
  return data ?? DEFAULT_MS;
}
