import { useNav, useViews } from "../queries/sidebar";
import { useSyncOverview } from "../queries/sync";
import { narrowingKey } from "../state/slices/narrowing";
import { useStore } from "../state/store";
import { ListView } from "../views/list/ListView";
import { TableView } from "../views/table/TableView";
import { SyncStatusView } from "./SyncStatusView";

/** The current view's issues, narrowed by the toolbar, once the cache holds any; before that, what the sync is
 *  doing. */
export function MainBody() {
  const { data: overview } = useSyncOverview();
  const { data: nav } = useNav();
  const location = nav?.location;
  const { data: views } = useViews(location?.listId ?? null);
  const narrowing = useStore((s) => (location ? s.narrowings[narrowingKey(location.listId, location.viewId)] : undefined));
  const hasIssues = overview?.projects.some((p) => p.cached > 0) ?? false;
  if (!hasIssues || !location) return <SyncStatusView />;
  const view = views?.find((v) => v.id === location.viewId);
  if (view?.layout === "table") return <TableView view={view} narrowing={narrowing} />;
  return <ListView listId={location.listId} viewId={location.viewId} narrowing={narrowing} />;
}
