import { ArrowDown, ArrowUp, Link2 } from "lucide-react";
import { useShortcuts } from "../commands/useShortcuts";
import { CopyKey } from "../components/issue/CopyKey";
import { useQueryClient } from "@tanstack/react-query";
import { type RefObject, useEffect, useMemo, useRef } from "react";
import { api, type IssueDetail } from "../api";
import { onIssueKeys } from "../commands/issueKeys";
import { SpaceTile } from "../components/SpaceTile";
import { hasContent } from "../lib/jiraHtml";
import { prefetchIssue, refreshIssueExtras, useIssue, useIssueRelated } from "../queries/issues";
import { useNavGo, useSidebar, useViews } from "../queries/sidebar";
import { useRequestNarrowing, useViewRows } from "../queries/views";
import { findLevel, spaceOf } from "../shell/levels";
import { narrowingKey } from "../state/slices/narrowing";
import type { IssueOrigin } from "../state/slices/issueView";
import { attempt } from "../state/requests";
import { useStore } from "../state/store";
import { useIssueActions } from "../views/list/useIssueActions";
import { Activity } from "./Activity";
import { Attachments } from "./Attachments";
import { flatten } from "../views/list/flatten";
import { GoneNotice } from "./GoneNotice";
import { IssueMissing } from "./IssueMissing";
import { SinceBanner } from "./SinceBanner";
import { useSeenAfter } from "./useSeenAfter";
import { ActionButton, IssueActions, toggleRead } from "./IssueActions";
import { FieldGrid } from "./FieldGrid";
import { IssueFind } from "./find/IssueFind";
import { JiraHtml } from "./JiraHtml";
import { RichFields } from "./RichFields";
import { Children, Links } from "./Related";
import { PullRequests } from "./PullRequests";
import { WebLinks } from "./WebLinks";
import { explainWritesOff } from "../writes/runWrite";
import { assignMenuAnchor } from "../writes/assignMenu";
import { priorityMenuAnchor } from "../writes/priorityMenu";
import { EditableTitle, requestRename } from "../writes/EditableTitle";
import { statusMenuAnchor } from "../writes/statusMenu";

/** The issue's neighbors among the rows the origin view shows (open groups, expanded subtrees), for the
 *  previous/next arrows and J/K; an issue the view hides steps through all of its rows. */
function useNeighbors(origin: IssueOrigin, id: string): { previous: string | null; next: string | null } {
  const narrowing = useRequestNarrowing(origin.listId, origin.viewId);
  const { data } = useViewRows(origin.listId, origin.viewId, narrowing);
  const place = useStore((s) => s.places[narrowingKey(origin.listId, origin.viewId)]);
  const order = useMemo(() => {
    if (!data) return [];
    const toggled = place?.toggled ?? {};
    const shown = flatten(data, (group) => toggled[group.key] ?? !group.collapsedByDefault, place?.collapsedParents).flatMap((item) =>
      item.kind === "row" ? [item.row.id] : [],
    );
    return shown.includes(id) ? shown : data.groups.flatMap((group) => group.rowIds);
  }, [data, place, id]);
  const at = order.indexOf(id);
  if (at < 0) return { previous: null, next: null };
  return { previous: order[at - 1] ?? null, next: order[at + 1] ?? null };
}

/** The full Issue view (artboards 5 and 6): it fills the content panel; one layout for every issue type. */
export function IssueView({ id, origin }: { id: string; origin: IssueOrigin }) {
  // The previous issue stays on screen until the next one loads; its neighbors load ahead.
  const { data: issue, error, isPlaceholderData } = useIssue(id, { keepPrevious: true });
  const back = useStore((s) => s.issueBack);
  const neighbors = useNeighbors(origin, id);
  const client = useQueryClient();
  const columnsRef = useRef<HTMLDivElement>(null);
  useSeenAfter(id, !isPlaceholderData && (issue?.unread ?? false));

  useEffect(() => {
    // Comments may be newer in Jira than in the cache; a failure (offline) leaves the cached ones.
    void api.issueSyncComments(id).catch(() => {});
    refreshIssueExtras(id);
    void api.issueViewed(id).catch(() => {});
  }, [id]);
  useEffect(() => {
    for (const neighbor of [neighbors.previous, neighbors.next]) if (neighbor) prefetchIssue(client, neighbor);
  }, [client, neighbors.previous, neighbors.next]);

  return (
    <div className="issue-view">
      <IssueHeader issue={issue} origin={origin} neighbors={neighbors} onBack={back} findIn={columnsRef} />
      <div ref={columnsRef} className="issue-view__columns">
        <div className="issue-view__main">
          {issue ? <IssueMain issue={issue} origin={origin} /> : error ? <IssueMissing error={error} className="issue-view__missing" /> : null}
        </div>
        <Activity id={id} />
      </div>
      <IssueKeys issue={isPlaceholderData ? undefined : issue} neighbors={neighbors} />
    </div>
  );
}

function IssueHeader({
  issue,
  origin,
  neighbors,
  onBack,
  findIn,
}: {
  issue: IssueDetail | undefined;
  origin: IssueOrigin;
  neighbors: { previous: string | null; next: string | null };
  onBack: () => void;
  findIn: RefObject<HTMLElement | null>;
}) {
  const { data: tree } = useSidebar();
  const { data: views } = useViews(origin.listId);
  const level = tree ? findLevel(tree, origin.listId) : null;
  const space = spaceOf(level);
  const originName = level?.kind === "list" || level?.kind === "home" ? level.node.name : views?.find((v) => v.id === origin.viewId)?.name;
  const showIssue = useStore((s) => s.showIssue);
  const actions = useIssueActions();
  const go = useNavGo();
  const { title } = useShortcuts();
  return (
    <header className="issue-header">
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
        {issue && <CopyKey className="issue-header__key" issueKey={issue.key} />}
        {issue && actions.openInJira && (
          <button type="button" className="issue-header__copy" aria-label={`Copy the link to ${issue.key}`} title={title("Copy link", "issue.copyUrl")} onClick={() => actions.copyUrl(issue.key)}>
            <Link2 size={13} strokeWidth={2} aria-hidden="true" />
          </button>
        )}
      </nav>
      <span className="issue-header__spacer" />
      <IssueFind rootRef={findIn} />
      <IssueActions
        issue={issue}
        onClose={onBack}
        closeCommand="issue.back"
        before={
          <>
            <ActionButton label="Previous issue" command="issue.previous" icon={ArrowUp} disabled={!neighbors.previous} onClick={() => neighbors.previous && showIssue(neighbors.previous)} />
            <ActionButton label="Next issue" command="issue.next" icon={ArrowDown} disabled={!neighbors.next} onClick={() => neighbors.next && showIssue(neighbors.next)} />
          </>
        }
      />
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
        <CopyKey className="issue-main__key" issueKey={issue.key} />
        <span aria-hidden="true">·</span>
        <span>{comments}</span>
        {files > 0 && (
          <>
            <span aria-hidden="true">·</span>
            <span>{files === 1 ? "1 attachment" : `${files} attachments`}</span>
          </>
        )}
      </div>
      <EditableTitle key={issue.id} issue={issue} as="h1" className="issue-main__title" />
      {issue.gone && <GoneNotice issue={issue} onRemoved={closeIssue} />}
      {issue.since && <SinceBanner since={issue.since} onMarkSeen={() => void attempt(api.issuesMarkSeen([issue.id]), "Could not mark it read")} />}
      <FieldGrid issue={issue} onOpenEpic={openRelated} />
      <PullRequests id={issue.id} variant="issue" />
      {related && <Links links={related.links} onOpen={openRelated} />}
      <section className="issue-main__description" aria-label="Description">
        <h2>Description</h2>
        {issue.descriptionHtml && hasContent(issue.descriptionHtml) ? (
          <JiraHtml html={issue.descriptionHtml} issueId={issue.id} />
        ) : (
          <p className="issue-main__empty">No description.</p>
        )}
      </section>
      <RichFields fields={issue.custom} issueId={issue.id} variant="issue" />
      <WebLinks id={issue.id} variant="issue" />
      {related && <Children related={related} isEpic={issue.isEpic} onOpen={openRelated} onOpenAsList={openAsList} />}
      <Attachments issueId={issue.id} attachments={issue.attachments} />
    </div>
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
    const withIssue = (run: (issue: IssueDetail) => void) => {
      const issue = state.current.issue;
      if (issue) run(issue);
    };
    const withKey = (run: (key: string) => void) => withIssue((issue) => run(issue.key));
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
      copyAppLink: () => withKey((key) => state.current.actions.copyAppLink(key)),
      copyMarkdown: () => withIssue((issue) => state.current.actions.copyMarkdown(issue)),
      openInJira: () => withKey((key) => state.current.actions.openInJira?.(key)),
      openEpic: () => {
        const epic = state.current.issue?.epic;
        if (epic) state.current.openRelated(epic.id);
      },
      togglePin: () => {
        const id = state.current.issue?.id;
        if (id) state.current.actions.togglePin(id);
      },
      toggleMine: () => {
        const id = state.current.issue?.id;
        if (id) (state.current.actions.toggleMine ?? explainWritesOff)(id);
      },
      changeStatus: () => {
        const { issue, actions } = state.current;
        if (!issue) return;
        if (actions.changeStatus) actions.changeStatus(issue, statusMenuAnchor(issue.id));
        else explainWritesOff();
      },
      assignTo: () => {
        const { issue, actions } = state.current;
        if (!issue) return;
        if (actions.assignTo) actions.assignTo(issue, assignMenuAnchor(issue.id));
        else explainWritesOff();
      },
      rename: () => {
        const { issue, actions } = state.current;
        if (!issue) return;
        if (actions.canChangeTitle) requestRename(issue.id);
        else explainWritesOff();
      },
      changePriority: () => {
        const { issue, actions } = state.current;
        if (!issue) return;
        if (actions.changePriority) actions.changePriority(issue, priorityMenuAnchor(issue.id));
        else explainWritesOff();
      },
      toggleRead: () => withIssue(toggleRead),
      hasSelection: () => state.current.issue !== undefined,
      peekOpen: () => false,
    });
  }, []);
  return null;
}
