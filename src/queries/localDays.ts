import { useQuery } from "@tanstack/react-query";
import { api, type LocalDays } from "../api";

export const LOCAL_DAYS_KEY = ["localDays"] as const;

/** The owner's day boundaries from Rust (ADR 0074): read again once tomorrow has begun, and when the settings (the
 *  time zone) change. */
export function useLocalDays(): LocalDays | undefined {
  const { data } = useQuery({
    queryKey: LOCAL_DAYS_KEY,
    queryFn: api.localDays,
    staleTime: Infinity,
    refetchInterval: (query) => {
      const tomorrow = query.state.data?.tomorrow;
      return tomorrow ? Math.max(1000, Date.parse(tomorrow) - Date.now()) : false;
    },
  });
  return data;
}
