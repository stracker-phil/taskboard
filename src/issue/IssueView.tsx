import { ArrowDown, ArrowLeft, ArrowUp, CheckCheck, CircleDot, ExternalLink, Link2 } from "lucide-react";
import { useEffect, useRef } from "react";
import { api, type IssueDetail } from "../api";
import { onIssueKeys } from "../commands/issueKeys";
import { SpaceTile } from "../components/SpaceTile";
import { hasContent } from "../lib/jiraHtml";
import { formatLocalTime } from "../lib/time";
import { refreshIssueExtras, useIssue, useIssueRelated } from "../queries/issues";
import { useNavGo, useSidebar, useViews } from "../queries/sidebar";
import { useViewRows } from "../queries/views";
import { findLevel, spaceOf } from "../shell/levels";
import { narrowingKey } from "../state/slices/narrowing";
import type { IssueOrigin } from "../state/slices/issueView";
import { attempt } from "../state/requests";
import { useStore } from "../state/store";
import { PinButton } from "../pins/PinButton";
import { useIssueActions } from "../views/list/useIssueActions";
import { Activity } from "./Activity";
import { Attachments } from "./Attachments";
import { GoneNotice } from "./GoneNotice";
import { SinceBanner } from "./SinceBanner";
import { holdUnread, useSeenAfter } from "./useSeenAfter";
import { FieldGrid } from "./FieldGrid";
import { JiraHtml } from "./JiraHtml";
import { RichFields } from "./RichFields";
import { Children, Links } from "./Related";
import { WebLinks } from "./WebLinks";

/** The issue's neighbors in the view it was opened from, for the previous/next arrows and J/K. */
function useNeighbors(origin: IssueOrigin, id: string): { previous: string | null; next: string | null } {
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
  const neighbors = useNeighbors(origin, id);
  useSeenAfter(id, issue?.unread ?? false);

  useEffect(() => {
    // Comments may be newer in Jira than in the cache; a failure (offline) leaves the cached ones.
    void api.issueSyncComments(id).catch(() => {});
    refreshIssueExtras(id);
  }, [id]);

  return (
    <div className="issue-view">
      <IssueHeader issue={issue} origin={origin} neighbors={neighbors} onBack={back} />
      <div className="issue-view__columns">
        <div className="issue-view__main">
          {issue ? <IssueMain issue={issue} origin={origin} /> : isError ? <p className="issue-view__missing">This issue is no longer in the cache.</p> : null}
        </div>
        <Activity id={id} />
      </div>
      <IssueKeys issue={issue} neighbors={neighbors} />
    </div>
  );
}

function IssueHeader({
  issue,
  origin,
  neighbors,
  onBack,
}: {
  issue: IssueDetail | undefined;
  origin: IssueOrigin;
  neighbors: { previous: string | null; next: string | null };
  onBack: () => void;
}) {
  const { data: tree } = useSidebar();
  const { data: views } = useViews(origin.listId);
  const level = tree ? findLevel(tree, origin.listId) : null;
  const space = spaceOf(level);
  const originName = level?.kind === "list" || level?.kind === "home" ? level.node.name : views?.find((v) => v.id === origin.viewId)?.name;
  const showIssue = useStore((s) => s.showIssue);
  const actions = useIssueActions();
  const go = useNavGo();
  return (
    <header className="issue-header">
      <button type="button" className="issue-header__icon" aria-label={`Back to ${originName ?? "the list"}`} title="Back (esc)" onClick={onBack}>
        <ArrowLeft size={16} strokeWidth={2} aria-hidden="true" />
      </button>
      <nav className="issue-header__crumbs" aria-label="Breadcrumb">
        {space && (
          <>
            <button type="button" className="issue-header__crumb" onClick={() => void go(space.listId)}>
              <SpaceTile space={space} size={20} />
              <span>{space.name}</span>
            </button>
            <span className="issue-header__sep">/</span>
          </>
        )}
        {originName && (
          <>
            <button type="button" className="issue-header__crumb" onClick={() => void go(origin.listId, origin.viewId)}>
              <span>{originName}</span>
            </button>
            <span className="issue-header__sep">/</span>
          </>
        )}
        <span className="issue-header__key">{issue?.key}</span>
        {issue && actions.openInJira && (
          <button type="button" className="issue-header__copy" aria-label={`Copy the link to ${issue.key}`} title="Copy link (⇧⌘C)" onClick={() => actions.copyUrl(issue.key)}>
            <Link2 size={13} strokeWidth={2} aria-hidden="true" />
          </button>
        )}
      </nav>
      <span className="issue-header__spacer" />
      {issue && <span className="issue-header__updated">Updated {formatLocalTime(issue.updatedAt)}</span>}
      <button type="button" className="issue-header__nav" aria-label="Previous issue" title="Previous (K)" disabled={!neighbors.previous} onClick={() => neighbors.previous && showIssue(neighbors.previous)}>
        <ArrowUp size={14} strokeWidth={2} aria-hidden="true" />
      </button>
      <button type="button" className="issue-header__nav" aria-label="Next issue" title="Next (J)" disabled={!neighbors.next} onClick={() => neighbors.next && showIssue(neighbors.next)}>
        <ArrowDown size={14} strokeWidth={2} aria-hidden="true" />
      </button>
      {issue && <ReadToggle issue={issue} />}
      {issue && <PinButton kind="issue" target={issue.id} label="Bookmark this issue" meta={issue.key} className="issue-header__nav" />}
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
  const files = issue.attachments.length;
  return (
    <div ref={scrollRef} className="issue-main">
      <div className="issue-main__type">
        <span className="type-badge">{issue.issueType}</span>
        <span className="issue-main__key">{issue.key}</span>
        <span aria-hidden="true">·</span>
        <span>{comments}</span>
        {files > 0 && (
          <>
            <span aria-hidden="true">·</span>
            <span>{files === 1 ? "1 attachment" : `${files} attachments`}</span>
          </>
        )}
      </div>
      <h1 className={issue.gone ? "issue-main__title issue-title--gone" : "issue-main__title"}>{issue.summary}</h1>
      {issue.gone && <GoneNotice issue={issue} onRemoved={closeIssue} />}
      {issue.since && <SinceBanner since={issue.since} onMarkSeen={() => void attempt(api.issuesMarkSeen([issue.id]), "Could not mark it read")} />}
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
      <RichFields fields={issue.custom} variant="issue" />
      <WebLinks id={issue.id} variant="issue" />
      {related && <Children related={related} isEpic={issue.isEpic} onOpen={openRelated} onOpenAsList={openAsList} />}
      <Attachments issueId={issue.id} attachments={issue.attachments} />
    </div>
  );
}

/** Read marks it read now; unread keeps it unread while it stays open. */
function toggleRead(issue: IssueDetail) {
  if (issue.unread) {
    void attempt(api.issuesMarkSeen([issue.id]), "Could not mark it read");
    return;
  }
  holdUnread(issue.id);
  void attempt(api.issueMarkUnread(issue.id), "Could not mark it unread");
}

function ReadToggle({ issue }: { issue: IssueDetail }) {
  const label = issue.unread ? "Mark as read" : "Mark as unread";
  const Icon = issue.unread ? CheckCheck : CircleDot;
  return (
    <button type="button" className="issue-header__nav" aria-label={label} title={`${label} (M)`} onClick={() => toggleRead(issue)}>
      <Icon size={14} strokeWidth={2} aria-hidden="true" />
    </button>
  );
}

/** J/K move to the neighbors; E opens the epic; ⌘C, ⌘⇧C and ⌘↩ act on the shown issue. */
function IssueKeys({ issue, neighbors }: { issue: IssueDetail | undefined; neighbors: { previous: string | null; next: string | null } }) {
  const showIssue = useStore((s) => s.showIssue);
  const openRelated = useStore((s) => s.openRelated);
  const actions = useIssueActions();
  const state = useRef({ issue, neighbors, showIssue, openRelated, actions });
  useEffect(() => {
    state.current = { issue, neighbors, showIssue, openRelated, actions };
  });
  useEffect(() => {
    const withKey = (run: (key: string) => void) => {
      const key = state.current.issue?.key;
      if (key) run(key);
    };
    return onIssueKeys({
      move: (step) => {
        const { neighbors, showIssue } = state.current;
        const target = step > 0 ? neighbors.next : neighbors.previous;
        if (target) showIssue(target);
      },
      setGroupOpen: () => {},
      closePeek: () => {},
      copyKey: () => withKey((key) => state.current.actions.copyKey(key)),
      copyUrl: () => withKey((key) => state.current.actions.copyUrl(key)),
      openInJira: () => withKey((key) => state.current.actions.openInJira?.(key)),
      openEpic: () => {
        const epic = state.current.issue?.epic;
        if (epic) state.current.openRelated(epic.id);
      },
      togglePin: () => {
        const id = state.current.issue?.id;
        if (id) state.current.actions.togglePin(id);
      },
      toggleRead: () => {
        const issue = state.current.issue;
        if (issue) toggleRead(issue);
      },
      hasSelection: () => state.current.issue !== undefined,
      peekOpen: () => false,
    });
  }, []);
  return null;
}
