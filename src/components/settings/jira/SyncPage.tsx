import { IntervalsGroup } from "./IntervalsGroup";
import { ScopeGroup } from "./ScopeGroup";
import { SyncedGroup } from "./SyncedGroup";

/** Settings → Sync: which issues Taskboard copies, how often, and what it holds per project. */
export function SyncPage() {
  return (
    <>
      <ScopeGroup />
      <IntervalsGroup />
      <SyncedGroup />
    </>
  );
}
