import { useQuery } from "@tanstack/react-query";
import { api, type Modes } from "../api";

export const MODES_KEY = ["modes"] as const;
const NO_MODES: Modes = { meEverywhere: false, unreadOnly: false, bookmarksBar: false };

/** The app-wide modes; off until Rust answers. */
export function useModes(): Modes {
  return useQuery({ queryKey: MODES_KEY, queryFn: api.modesGet }).data ?? NO_MODES;
}
