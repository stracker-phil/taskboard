import { IntervalsGroup } from "./IntervalsGroup";
import { IssueTypesGroup } from "./IssueTypesGroup";
import { ScopeGroup } from "./ScopeGroup";
import { SyncedGroup } from "./SyncedGroup";

/** Settings → Sync: which issues Taskboard copies, how often, and what it holds per project. */
export function SyncPage() {
  return (
    <>
      <ScopeGroup />
      <IssueTypesGroup />
      <IntervalsGroup />
      <SyncedGroup />
    </>
  );
}
