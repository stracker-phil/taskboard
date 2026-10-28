import { useEffect } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api, type PinItem, type PinKind, type Pinned } from "../api";
import { SIDEBAR_KEY } from "./sidebar";
import { VIEW_ROWS_KEY } from "./views";

export const PINS_KEY = ["pins"] as const;

export const usePins = () => useQuery({ queryKey: PINS_KEY, queryFn: api.pinsList });

const targetOf = (pinned: Pinned): { kind: PinKind; id: string } =>
  pinned.kind === "list" ? { kind: "filterList", id: String(pinned.id) } : { kind: pinned.kind, id: pinned.id };

/** The target's active pin, if it has one. */
export function usePinOf(kind: PinKind, id: string): PinItem | undefined {
  const { data } = usePins();
  return data?.active.find((pin) => {
    const target = targetOf(pin.target);
    return target.kind === kind && target.id === id;
  });
}

/** Refreshes pins, the sidebar and the Pinned view when the next pin runs out. */
export function usePinExpiry() {
  const client = useQueryClient();
  const { data } = usePins();
  const next = data?.active
    .map((pin) => (pin.expiresAt ? Date.parse(pin.expiresAt) : Number.POSITIVE_INFINITY))
    .reduce((a, b) => Math.min(a, b), Number.POSITIVE_INFINITY);
  useEffect(() => {
    if (next === undefined || !Number.isFinite(next)) return;
    // setTimeout holds at most ~24.8 days; a later expiry re-arms on the next refetch.
    const wait = Math.min(Math.max(next - Date.now(), 0) + 500, 2 ** 31 - 1);
    const timer = setTimeout(() => {
      for (const queryKey of [PINS_KEY, SIDEBAR_KEY, VIEW_ROWS_KEY]) void client.invalidateQueries({ queryKey });
    }, wait);
    return () => clearTimeout(timer);
  }, [client, next]);
}
