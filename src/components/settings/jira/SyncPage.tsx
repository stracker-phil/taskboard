import { CacheGroup } from "./CacheGroup";
import { IntervalsGroup } from "./IntervalsGroup";
import { ScopeGroup } from "./ScopeGroup";

/** Settings → Sync: which issues Taskboard copies, how often, and the cache. */
export function SyncPage() {
  return (
    <>
      <ScopeGroup />
      <IntervalsGroup />
      <CacheGroup />
    </>
  );
}
