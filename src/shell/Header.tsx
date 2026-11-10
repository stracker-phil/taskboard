import { OpenInJiraButton } from "../components/OpenInJiraButton";
import { browseUrl } from "../lib/jiraLinks";
import { api, type SpaceNode } from "../api";
import { useConnection } from "../queries/settings";
import { attempt } from "../state/requests";
import { PinButton } from "../pins/PinButton";
import { HeaderTitle } from "./HeaderTitle";
import { spaceOf, type Level } from "./levels";

/** The content header: the level's tile or icon and name (a filter list as Space / folder / name) and its bookmark,
 *  then Open in Jira for a Space and its lists. */
export function Header({ level }: { level: Level }) {
  return (
    <header className="content-header">
      <HeaderTitle key={level.kind === "space" ? level.space.listId : level.node.id} level={level} />
      <LevelBookmark level={level} />
      <div className="content-header__spacer" />
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

/** Jira's page of the Space's project (`/browse/KEY` redirects to it); hidden without a Space or a site. */
function OpenInJira({ space }: { space: SpaceNode | null }) {
  const { data: connection } = useConnection();
  const url = space ? browseUrl(connection?.connection.siteUrl, space.key) : null;
  if (!url) return null;
  return <OpenInJiraButton onClick={() => void attempt(api.openUrl(url), "Could not open Jira")} />;
}
