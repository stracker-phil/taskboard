import { PinnedView } from "../pins/PinnedView";
import { useNav, useSidebar, useViews } from "../queries/sidebar";
import { useSyncOverview } from "../queries/sync";
import { narrowingKey } from "../state/slices/narrowing";
import { useStore } from "../state/store";
import { BoardView } from "../views/board/BoardView";
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
  const { data: tree } = useSidebar();
  const narrowing = useStore((s) => (location ? s.narrowings[narrowingKey(location.listId, location.viewId)] : undefined));
  const hasIssues = overview?.projects.some((p) => p.cached > 0) ?? false;
  if (!hasIssues || !location) return <SyncStatusView />;
  // A Space still in its first sync shows the progress; finished Spaces can be browsed meanwhile.
  const space = tree?.spaces.find((s) => s.listId === location.listId || s.lists.some((l) => l.id === location.listId));
  const loading = overview?.state.state === "running" && overview.projects.some((p) => p.id === space?.projectId && !p.lastFullAt);
  if (loading) return <SyncStatusView />;
  if (tree?.home.some((node) => node.id === location.listId && node.kind === "pinned")) {
    return <PinnedView listId={location.listId} viewId={location.viewId} />;
  }
  const view = views?.find((v) => v.id === location.viewId);
  // Until the views load the layout is unknown; drawing a List first would flash before a Board or Table.
  if (!view) return null;
  if (view.layout === "table") return <TableView view={view} narrowing={narrowing} />;
  if (view.layout === "board") return <BoardView view={view} narrowing={narrowing} />;
  return <ListView listId={location.listId} viewId={location.viewId} narrowing={narrowing} wrapTitles={view.wrapTitles} />;
}
