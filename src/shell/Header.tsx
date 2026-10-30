import { usePageAction } from "../commands/pageActions";
import { OpenInJiraButton } from "../components/OpenInJiraButton";
import { browseUrl } from "../lib/jiraLinks";
import { api, type SpaceNode } from "../api";
import { useConnection } from "../queries/settings";
import { useNav } from "../queries/sidebar";
import { useRequestNarrowing } from "../queries/views";
import { attempt } from "../state/requests";
import { PinButton } from "../pins/PinButton";
import { HeaderTitle } from "./HeaderTitle";
import { spaceOf, type Level } from "./levels";

/** The content header: the level's tile or icon and name (a filter list as Space / folder / name) and its bookmark,
 *  then Mark all as read for Unread and Open in Jira for a Space and its lists. */
export function Header({ level }: { level: Level }) {
  return (
    <header className="content-header">
      <HeaderTitle key={level.kind === "space" ? level.space.listId : level.node.id} level={level} />
      <LevelBookmark level={level} />
      <div className="content-header__spacer" />
      {level.kind === "home" && level.node.kind === "recent" && <MarkAllSeen listId={level.node.id} unseen={level.node.count ?? 0} />}
      <OpenInJira space={spaceOf(level)} />
    </header>
  );
}

/** Beside the name of a Space or filter list: bookmarks it (and adds it to Home). Always there, so it never
 *  appears under the cursor; it takes the bookmark color while the level is bookmarked. */
function LevelBookmark({ level }: { level: Level }) {
  if (level.kind === "home" && level.node.kind !== "list") return null;
  const id = level.kind === "space" ? level.space.listId : level.node.id;
  const name = level.kind === "space" ? level.space.name : level.node.name;
  return <PinButton kind="filterList" target={String(id)} label={level.kind === "space" ? "Bookmark this Space" : "Bookmark this list"} meta={name} className="content-header__pin" iconSize={16} />;
}

/** Unread's header action: marks what the current view shows read; disabled while nothing is unread. */
function MarkAllSeen({ listId, unseen }: { listId: number; unseen: number }) {
  const { data: nav } = useNav();
  const viewId = nav?.location?.listId === listId ? nav.location.viewId : null;
  const narrowing = useRequestNarrowing(listId, viewId ?? -1) ?? null;
  const markAll = () => void attempt(api.listMarkAllSeen(listId, viewId, viewId === null ? null : narrowing), "Could not mark them read");
  usePageAction("recent.markAllSeen", unseen > 0 ? markAll : null);
  return (
    <button type="button" className="content-header__action" disabled={unseen === 0} onClick={markAll}>
      Mark all as read
    </button>
  );
}

/** Jira's page of the Space's project (`/browse/KEY` redirects to it); hidden without a Space or a site. */
function OpenInJira({ space }: { space: SpaceNode | null }) {
  const { data: connection } = useConnection();
  const url = space ? browseUrl(connection?.connection.siteUrl, space.key) : null;
  if (!url) return null;
  return <OpenInJiraButton onClick={() => void attempt(api.openUrl(url), "Could not open Jira")} />;
}
