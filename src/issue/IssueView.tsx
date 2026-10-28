import { ArrowDown, ArrowLeft, ArrowUp, ExternalLink } from "lucide-react";
import { useEffect, useRef } from "react";
import { api, type IssueDetail } from "../api";
import { onIssueKeys } from "../commands/issueKeys";
import { SpaceTile } from "../components/SpaceTile";
import { hasContent } from "../lib/jiraHtml";
import { formatLocalTime } from "../lib/time";
import { useIssue, useIssueRelated } from "../queries/issues";
import { useSidebar, useViews } from "../queries/sidebar";
import { useViewRows } from "../queries/views";
import { findLevel, spaceOf } from "../shell/levels";
import { narrowingKey } from "../state/slices/narrowing";
import type { IssueOrigin } from "../state/slices/issueView";
import { attempt } from "../state/requests";
import { useStore } from "../state/store";
import { PinButton } from "../pins/PinButton";
import { useIssueActions } from "../views/list/useIssueActions";
import { Activity } from "./Activity";
import { GoneNotice } from "./GoneNotice";
import { SinceBanner } from "./SinceBanner";
import { useSeenAfter } from "./useSeenAfter";
import { FieldGrid } from "./FieldGrid";
import { JiraHtml } from "./JiraHtml";
import { Children, Links } from "./Related";

/** The issue's neighbours in the view it was opened from, for the previous/next arrows and J/K. */
function useNeighbours(origin: IssueOrigin, id: string): { previous: string | null; next: string | null } {
  const narrowing = useStore((s) => s.narrowings[narrowingKey(origin.listId, origin.viewId)]);
  const { data } = useViewRows(origin.listId, origin.viewId, narrowing);
  const order = data?.groups.flatMap((group) => group.rowIds) ?? [];
  const at = order.indexOf(id);
  if (at < 0) return { previous: null, next: null };
  return { previous: order[at - 1] ?? null, next: order[at + 1] ?? null };
}

/** The full Issue view (artboards 5 and 6): it fills the content panel; one layout for every issue type. */
export function IssueView({ id, origin }: { id: string; origin: IssueOrigin }) {
  const { data: issue, isError } = useIssue(id);
  const back = useStore((s) => s.issueBack);
  const neighbours = useNeighbours(origin, id);
  useSeenAfter(id, Boolean(issue?.since));

  useEffect(() => {
    // Comments may be newer in Jira than in the cache; a failure (offline) leaves the cached ones.
    void api.issueSyncComments(id).catch(() => {});
  }, [id]);

  return (
    <div className="issue-view">
      <IssueHeader issue={issue} origin={origin} neighbours={neighbours} onBack={back} />
      <div className="issue-view__columns">
        <div className="issue-view__main">
          {issue ? <IssueMain issue={issue} origin={origin} /> : isError ? <p className="issue-view__missing">This issue is no longer in the cache.</p> : null}
        </div>
        <Activity id={id} />
      </div>
      <IssueKeys issue={issue} neighbours={neighbours} />
    </div>
  );
}

function IssueHeader({
  issue,
  origin,
  neighbours,
  onBack,
}: {
  issue: IssueDetail | undefined;
  origin: IssueOrigin;
  neighbours: { previous: string | null; next: string | null };
  onBack: () => void;
}) {
  const { data: tree } = useSidebar();
  const { data: views } = useViews(origin.listId);
  const level = tree ? findLevel(tree, origin.listId) : null;
  const space = spaceOf(level);
  const originName = level?.kind === "list" || level?.kind === "home" ? level.node.name : views?.find((v) => v.id === origin.viewId)?.name;
  const showIssue = useStore((s) => s.showIssue);
  const actions = useIssueActions();
  return (
    <header className="issue-header">
      <button type="button" className="issue-header__icon" aria-label={`Back to ${originName ?? "the list"}`} title="Back (esc)" onClick={onBack}>
        <ArrowLeft size={16} strokeWidth={2} aria-hidden="true" />
      </button>
      <nav className="issue-header__crumbs" aria-label="Breadcrumb">
        {space && (
          <>
            <SpaceTile space={space} size={20} />
            <span>{space.name}</span>
            <span className="issue-header__sep">/</span>
          </>
        )}
        {originName && (
          <>
            <span>{originName}</span>
            <span className="issue-header__sep">/</span>
          </>
        )}
        <span className="issue-header__key">{issue?.key}</span>
      </nav>
      <span className="issue-header__spacer" />
      {issue && <span className="issue-header__updated">Updated {formatLocalTime(issue.updatedAt)}</span>}
      <button type="button" className="issue-header__nav" aria-label="Previous issue" title="Previous (K)" disabled={!neighbours.previous} onClick={() => neighbours.previous && showIssue(neighbours.previous)}>
        <ArrowUp size={14} strokeWidth={2} aria-hidden="true" />
      </button>
      <button type="button" className="issue-header__nav" aria-label="Next issue" title="Next (J)" disabled={!neighbours.next} onClick={() => neighbours.next && showIssue(neighbours.next)}>
        <ArrowDown size={14} strokeWidth={2} aria-hidden="true" />
      </button>
      {issue && <PinButton kind="issue" target={issue.id} label="Pin this issue" meta={issue.key} className="issue-header__nav" />}
      {issue && actions.openInJira && (
        <button type="button" className="btn-primary-dark issue-header__jira" onClick={() => actions.openInJira?.(issue.key)}>
          Open in Jira
          <ExternalLink size={13} strokeWidth={2} aria-hidden="true" />
        </button>
      )}
    </header>
  );
}

function IssueMain({ issue, origin }: { issue: IssueDetail; origin: IssueOrigin }) {
  const openRelated = useStore((s) => s.openRelated);
  const setNarrowing = useStore((s) => s.setNarrowing);
  const closeIssue = useStore((s) => s.closeIssue);
  const { data: related } = useIssueRelated(issue.id);
  const scrollRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (scrollRef.current) scrollRef.current.scrollTop = 0;
  }, [issue.id]);
  // The epic's issues in the view it came from, narrowed like the Me chip: shown in the toolbar, never saved.
  const openAsList = () => {
    setNarrowing(narrowingKey(origin.listId, origin.viewId), { epic: issue.id });
    closeIssue();
  };
  const comments = issue.commentCount === 1 ? "1 comment" : `${issue.commentCount} comments`;
  return (
    <div ref={scrollRef} className="issue-main">
      <div className="issue-main__type">
        <span className="type-badge">{issue.issueType}</span>
        <span className="issue-main__key">{issue.key}</span>
        <span aria-hidden="true">·</span>
        <span>{comments}</span>
      </div>
      <h1 className={issue.gone ? "issue-main__title issue-title--gone" : "issue-main__title"}>{issue.summary}</h1>
      {issue.gone && <GoneNotice issue={issue} onRemoved={closeIssue} />}
      {issue.since && <SinceBanner since={issue.since} onMarkSeen={() => void attempt(api.issuesMarkSeen([issue.id]), "Could not mark it seen")} />}
      <FieldGrid issue={issue} onOpenEpic={openRelated} />
      {related && <Links links={related.links} onOpen={openRelated} />}
      <section className="issue-main__description" aria-label="Description">
        <h2>Description</h2>
        {issue.descriptionHtml && hasContent(issue.descriptionHtml) ? (
          <JiraHtml html={issue.descriptionHtml} />
        ) : (
          <p className="issue-main__empty">No description.</p>
        )}
      </section>
      {related && <Children related={related} isEpic={issue.isEpic} onOpen={openRelated} onOpenAsList={openAsList} />}
    </div>
  );
}

/** J/K move to the neighbours; E opens the epic; ⌘C, ⌘⇧C and ⌘↩ act on the shown issue. */
function IssueKeys({ issue, neighbours }: { issue: IssueDetail | undefined; neighbours: { previous: string | null; next: string | null } }) {
  const showIssue = useStore((s) => s.showIssue);
  const openRelated = useStore((s) => s.openRelated);
  const actions = useIssueActions();
  const state = useRef({ issue, neighbours, showIssue, openRelated, actions });
  useEffect(() => {
    state.current = { issue, neighbours, showIssue, openRelated, actions };
  });
  useEffect(() => {
    const withKey = (run: (key: string) => void) => {
      const key = state.current.issue?.key;
      if (key) run(key);
    };
    return onIssueKeys({
      move: (step) => {
        const { neighbours, showIssue } = state.current;
        const target = step > 0 ? neighbours.next : neighbours.previous;
        if (target) showIssue(target);
      },
      setGroupOpen: () => {},
      closePeek: () => {},
      copyKey: () => withKey((key) => state.current.actions.copyKey(key)),
      copyUrl: () => withKey((key) => state.current.actions.copyUrl(key)),
      openInJira: () => withKey((key) => state.current.actions.openInJira?.(key)),
      open: () => {},
      openEpic: () => {
        const epic = state.current.issue?.epic;
        if (epic) state.current.openRelated(epic.id);
      },
      togglePin: () => {
        const id = state.current.issue?.id;
        if (id) state.current.actions.togglePin(id);
      },
      hasSelection: () => state.current.issue !== undefined,
      peekOpen: () => false,
    });
  }, []);
  return null;
}
