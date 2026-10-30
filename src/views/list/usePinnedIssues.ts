import { useMemo } from "react";
import { usePins } from "../../queries/pins";

const NONE: ReadonlySet<string> = new Set();

/** The ids of the issues with an active bookmark, looked up once for all rows. */
export function usePinnedIssues(): ReadonlySet<string> {
  const { data } = usePins();
  return useMemo(() => {
    if (!data) return NONE;
    return new Set(data.active.flatMap((pin) => (pin.target.kind === "issue" ? [pin.target.id] : [])));
  }, [data]);
}
