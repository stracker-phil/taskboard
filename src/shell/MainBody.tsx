import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import type { Narrowing, View } from "../api";
import { PinnedView } from "../pins/PinnedView";
import { useNav, useSidebar, useViews } from "../queries/sidebar";
import { useSyncOverview } from "../queries/sync";
import { requestNarrowing, viewDataQuery } from "../queries/views";
import { narrowingKey, NO_VIEW_NARROWING } from "../state/slices/narrowing";
import { useStore } from "../state/store";
import { BoardView } from "../views/board/BoardView";
import { ListView } from "../views/list/ListView";
import { TableView } from "../views/table/TableView";
import { SyncStatusView } from "./SyncStatusView";

/** What the body shows: the sync's progress, the Bookmarks page, or a view's layout. */
type Body = { kind: "status" } | { kind: "pinned"; listId: number; viewId: number } | { kind: "view"; view: View; narrowing: Narrowing };

const NO_VIEW_DATA = { queryKey: ["noViewData"], queryFn: () => Promise.resolve(null) };

const bodyKey = (body: Body) => (body.kind === "view" ? `view:${body.view.id}:${body.view.layout}:${JSON.stringify(body.narrowing)}` : body.kind === "pinned" ? `pinned:${body.listId}` : "status");

/** The current view's issues, narrowed by the toolbar, once the cache holds any; before that, what the sync is
 *  doing. A view shows once its data is loaded: until then the one before stays, so switching views, layouts or
 *  the narrowing changes the content in place rather than blanking it. */
export function MainBody() {
  const next = useNextBody();
  const viewData = next?.kind === "view" ? viewDataQuery(next.view, next.narrowing) : null;
  const { data } = useQuery({ ...(viewData ?? NO_VIEW_DATA), enabled: viewData !== null });
  const ready = next !== null && (next.kind !== "view" || data !== undefined);
  const [previous, setPrevious] = useState<Body | null>(null);
  const nextKey = next && ready ? bodyKey(next) : null;
  useEffect(() => {
    if (next && nextKey) setPrevious(next);
    // The key stands for the body: a new one only when what it shows changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [nextKey]);
  const body = ready ? next : previous;
  if (!body) return null;
  switch (body.kind) {
    case "status":
      return <SyncStatusView />;
    case "pinned":
      return <PinnedView listId={body.listId} viewId={body.viewId} />;
    case "view": {
      const { view, narrowing } = body;
      if (view.layout === "table") return <TableView view={view} narrowing={narrowing} />;
      if (view.layout === "board") return <BoardView view={view} narrowing={narrowing} />;
      return <ListView listId={view.listId} viewId={view.id} narrowing={narrowing} wrapTitles={view.wrapTitles} openWith={view.openWith} inlineLabels={view.inlineLabels} />;
    }
  }
}

/** What the body should show for the current place; null while the sync state, the place or its views load. */
function useNextBody(): Body | null {
  const { data: overview } = useSyncOverview();
  const { data: nav } = useNav();
  const location = nav?.location;
  const { data: views } = useViews(location?.listId ?? null);
  const { data: tree } = useSidebar();
  const loadPlace = useStore((s) => s.loadPlace);
  // Every layout's place loads here; the List also restores its scroll from it.
  useEffect(() => {
    if (location) void loadPlace(location.listId, location.viewId);
  }, [loadPlace, location]);
  const stored = useStore((s) => (location ? s.narrowings[narrowingKey(location.listId, location.viewId)] : undefined));
  // Until the sync state and the place are known nothing is ready, so the status never flashes at launch.
  if (!overview || !nav) return null;
  const hasIssues = overview.projects.some((p) => p.cached > 0);
  if (!hasIssues || !location) return { kind: "status" };
  // A Space still in its first sync shows the progress; finished Spaces can be browsed meanwhile.
  const space = tree?.spaces.find((s) => s.listId === location.listId || s.lists.some((l) => l.id === location.listId));
  const loading = overview.state.state === "running" && overview.projects.some((p) => p.id === space?.projectId && !p.lastFullAt);
  if (loading) return { kind: "status" };
  if (tree?.home.some((node) => node.id === location.listId && node.kind === "pinned")) return { kind: "pinned", listId: location.listId, viewId: location.viewId };
  const view = views?.find((v) => v.id === location.viewId);
  if (!view) return null;
  return { kind: "view", view, narrowing: requestNarrowing(stored ?? NO_VIEW_NARROWING, view.onlyMine) };
}
