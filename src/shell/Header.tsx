import { Funnel } from "lucide-react";
import { browseUrl } from "../lib/jiraLinks";
import { api, type SpaceNode } from "../api";
import { SpaceTile } from "../components/SpaceTile";
import { useConnection } from "../queries/settings";
import { useNavGo } from "../queries/sidebar";
import { attempt } from "../state/requests";
import { HOME_ICONS, spaceOf, type Level } from "./levels";

/** The content header: the level's tile or icon and name (a filter list as Space / funnel / name), then Mark all
 *  seen for Recently changed and Open in Jira for a Space and its lists. */
export function Header({ level }: { level: Level }) {
  return (
    <header className="content-header">
      <Title level={level} />
      <div className="content-header__spacer" />
      {level.kind === "home" && level.node.kind === "recent" && <MarkAllSeen listId={level.node.id} unseen={level.node.count ?? 0} />}
      <OpenInJira space={spaceOf(level)} />
    </header>
  );
}

function Title({ level }: { level: Level }) {
  const open = useNavGo();
  switch (level.kind) {
    case "space":
      return (
        <>
          <SpaceTile space={level.space} size={28} />
          <h1 className="content-header__title">{level.space.name}</h1>
        </>
      );
    case "list":
      return (
        <>
          <button type="button" className="content-header__crumb" onClick={() => void open(level.space.listId)}>
            <SpaceTile space={level.space} size={20} />
            {level.space.name}
          </button>
          <span className="content-header__slash" aria-hidden="true">
            /
          </span>
          <Funnel className="content-header__icon" size={15} strokeWidth={2} aria-hidden="true" />
          <h1 className="content-header__title">{level.node.name}</h1>
        </>
      );
    case "home": {
      const Icon = HOME_ICONS[level.node.kind];
      return (
        <>
          <Icon className="content-header__icon" size={20} strokeWidth={1.8} aria-hidden="true" />
          <h1 className="content-header__title">{level.node.name}</h1>
        </>
      );
    }
  }
}

/** Recently changed's header action; disabled while nothing is unseen. */
function MarkAllSeen({ listId, unseen }: { listId: number; unseen: number }) {
  return (
    <button type="button" className="content-header__action" disabled={unseen === 0} onClick={() => void attempt(api.listMarkAllSeen(listId), "Could not mark them seen")}>
      Mark all seen
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
