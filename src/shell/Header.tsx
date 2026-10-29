import { usePageAction } from "../commands/pageActions";
import { browseUrl } from "../lib/jiraLinks";
import { api, type SpaceNode } from "../api";
import { useConnection } from "../queries/settings";
import { attempt } from "../state/requests";
import { HeaderTitle } from "./HeaderTitle";
import { spaceOf, type Level } from "./levels";

/** The content header: the level's tile or icon and name (a filter list as Space / funnel / name), then Mark all
 *  as read for Unread and Open in Jira for a Space and its lists. */
export function Header({ level }: { level: Level }) {
  return (
    <header className="content-header">
      <HeaderTitle key={level.kind === "space" ? level.space.listId : level.node.id} level={level} />
      <div className="content-header__spacer" />
      {level.kind === "home" && level.node.kind === "recent" && <MarkAllSeen listId={level.node.id} unseen={level.node.count ?? 0} />}
      <OpenInJira space={spaceOf(level)} />
    </header>
  );
}

/** Unread's header action; disabled while nothing is unread. */
function MarkAllSeen({ listId, unseen }: { listId: number; unseen: number }) {
  const markAll = () => void attempt(api.listMarkAllSeen(listId), "Could not mark them read");
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
  return (
    <button type="button" className="content-header__jira" onClick={() => void attempt(api.openUrl(url), "Could not open Jira")}>
      Open in Jira <span aria-hidden="true">↗</span>
    </button>
  );
}
