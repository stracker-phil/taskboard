import { useNav } from "../queries/sidebar";
import { useSyncOverview } from "../queries/sync";
import { ListView } from "../views/list/ListView";
import { SyncStatusView } from "./SyncStatusView";

/** The current view's issues once the cache holds any; before that, what the sync is doing. */
export function MainBody() {
  const { data: overview } = useSyncOverview();
  const { data: nav } = useNav();
  const location = nav?.location;
  const hasIssues = overview?.projects.some((p) => p.cached > 0) ?? false;
  if (!hasIssues || !location) return <SyncStatusView />;
  return <ListView listId={location.listId} viewId={location.viewId} />;
}
