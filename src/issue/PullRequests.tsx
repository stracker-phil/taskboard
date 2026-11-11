import { useQueryClient } from "@tanstack/react-query";
import { CircleCheck, CircleDashed, CircleX, Clock, GitMerge, GitPullRequest, GitPullRequestClosed, GitPullRequestDraft, type LucideIcon } from "lucide-react";
import { useEffect } from "react";
import { api, type Checks, type IssuePullRequests, type PrState, type PrStatus, type PullRequest, type ReviewState } from "../api";
import { pullRequestsKey, useIssuePullRequests } from "../queries/issues";
import { attempt } from "../state/requests";

const STATE: Record<PrState, { label: string; icon: LucideIcon }> = {
  open: { label: "Open", icon: GitPullRequest },
  draft: { label: "Draft", icon: GitPullRequestDraft },
  merged: { label: "Merged", icon: GitMerge },
  closed: { label: "Closed", icon: GitPullRequestClosed },
};

const REVIEW: Record<Exclude<ReviewState, "none">, { label: string; icon: LucideIcon; tone: string }> = {
  approved: { label: "Approved", icon: CircleCheck, tone: "pass" },
  changesRequested: { label: "Changes requested", icon: CircleX, tone: "fail" },
  waiting: { label: "Awaiting review", icon: Clock, tone: "pending" },
};

const CHECKS: Record<Exclude<Checks["state"], "none">, { icon: LucideIcon; tone: string; words: string }> = {
  passing: { icon: CircleCheck, tone: "pass", words: "Checks pass" },
  failing: { icon: CircleX, tone: "fail", words: "Checks fail" },
  pending: { icon: CircleDashed, tone: "pending", words: "Checks running" },
};

/** The issue's pull requests with their GitHub state. */
export function PullRequests({ id, variant }: { id: string; variant: "issue" | "peek" }) {
  const { data } = useIssuePullRequests(id);
  useSyncWhenDue(id, data);
  const items = data?.items ?? [];
  if (items.length === 0) return null;
  const label = items.length === 1 ? "Pull request" : "Pull requests";
  return (
    <section className={`pull-requests pull-requests--${variant}`} aria-label="Pull requests">
      {variant === "peek" && <h2>Pull requests</h2>}
      {items.map((pr, i) => (
        <div key={pr.id} className="pull-requests__row">
          {variant === "issue" && <span className="pull-requests__label">{i === 0 ? label : ""}</span>}
          <PullRequestRow pr={pr} />
        </div>
      ))}
    </section>
  );
}

/** Rust answers with `refreshInMs` while a status may change; then it is asked to sync. A failed sync keeps the
 *  shown statuses until the issue is opened again. */
function useSyncWhenDue(id: string, data: IssuePullRequests | undefined) {
  const client = useQueryClient();
  useEffect(() => {
    const refreshIn = data?.refreshInMs;
    if (refreshIn === null || refreshIn === undefined) return;
    const timer = window.setTimeout(() => {
      void api
        .issueSyncPullRequests(id)
        .then((fresh) => client.setQueryData(pullRequestsKey(id), fresh))
        .catch(() => {});
    }, refreshIn);
    return () => window.clearTimeout(timer);
  }, [id, data, client]);
}

function PullRequestRow({ pr }: { pr: PullRequest }) {
  const Icon = pr.status ? STATE[pr.status.state].icon : GitPullRequest;
  const content = (
    <>
      <Icon size={14} strokeWidth={1.9} className={`pull-request__icon pull-request__icon--${pr.status?.state ?? "unknown"}`} aria-hidden="true" />
      <span className="pull-request__number">#{pr.number}</span>
      <span className="pull-request__title">{pr.title}</span>
      {pr.status ? <Status status={pr.status} /> : pr.problem && <span className="pull-request__problem" title={pr.problem}>No status</span>}
    </>
  );
  const tooltip = `${pr.repo}#${pr.number}`;
  if (!pr.openable) {
    return (
      <div className="pull-request pull-request--inert" title={tooltip}>
        {content}
      </div>
    );
  }
  return (
    <button type="button" className="pull-request" title={`${tooltip} · Open on the web`} onClick={() => void attempt(api.openUrl(pr.url), "Could not open the link")}>
      {content}
    </button>
  );
}

function Status({ status }: { status: PrStatus }) {
  const review = status.review === "none" ? null : REVIEW[status.review];
  return (
    <span className="pull-request__status">
      <ChecksMark checks={status.checks} />
      {review && (
        <span className={`pull-request__mark pull-request__mark--${review.tone}`}>
          <review.icon size={13} strokeWidth={2} aria-hidden="true" />
          {review.label}
        </span>
      )}
      <span className={`pull-request__state pull-request__state--${status.state}`}>{STATE[status.state].label}</span>
    </span>
  );
}

function ChecksMark({ checks }: { checks: Checks }) {
  if (checks.state === "none") return null;
  const { icon: Icon, tone, words } = CHECKS[checks.state];
  return (
    <span className={`pull-request__mark pull-request__mark--${tone}`} title={`${words}: ${checks.passed} of ${checks.total} passed`}>
      <Icon size={13} strokeWidth={2} aria-hidden="true" />
      {checks.passed}/{checks.total}
    </span>
  );
}
