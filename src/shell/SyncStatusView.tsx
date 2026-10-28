import { api, type ProjectCounts, type SyncOverview, type SyncProgress } from "../api";
import { formatLocalTime } from "../lib/time";
import { describeSyncState, useSyncOverview, useSyncProgress } from "../queries/sync";
import { fail } from "../state/store";

/** The content panel until the issue list arrives: connect, loading per project, or what the cache holds. */
export function SyncStatusView() {
  const { data: overview } = useSyncOverview();
  const progress = useSyncProgress();
  if (!overview) return null;
  return <div className="content-panel__placeholder">{body(overview, progress)}</div>;
}

function body(overview: SyncOverview, progress: Record<string, SyncProgress>) {
  const { state, projects } = overview;
  if (projects.length === 0) return <ConnectPrompt />;
  switch (state.state) {
    case "running":
      return <FirstSync projects={projects} progress={progress} />;
    case "idle": {
      const total = projects.reduce((sum, p) => sum + p.cached, 0);
      const spaces = projects.length === 1 ? "1 Space" : `${projects.length} Spaces`;
      return (
        <div className="sync-status">
          <p>
            {total === 1 ? "1 issue" : `${total} issues`} in {spaces}
          </p>
          {overview.lastFullAt && <p className="sync-status__detail">Synced at {formatLocalTime(overview.lastFullAt)}</p>}
        </div>
      );
    }
    default:
      return <p>{describeSyncState(state)}</p>;
  }
}

function ConnectPrompt() {
  const openSettings = () => void api.runCommand("app:settings").catch((e: unknown) => fail(e, "Could not open Settings"));
  return (
    <div className="sync-status">
      <p>Connect Jira in Settings</p>
      <button type="button" className="btn" onClick={openSettings}>
        Open Settings
      </button>
    </div>
  );
}

/** The first full sync (artboard 8, card 1): an overall bar and every Space done, loading or waiting. */
export function FirstSync({ projects, progress }: { projects: ProjectCounts[]; progress: Record<string, SyncProgress> }) {
  const rows = projects.map((project) => ({ project, state: projectState(project, progress[project.id]), progress: progress[project.id] }));
  // Done Spaces count what they hold; the others what they fetched of Jira's estimate.
  const fetched = rows.reduce((sum, r) => sum + (r.state === "done" ? r.project.cached : (r.progress?.fetched ?? 0)), 0);
  const known = rows.every((r) => r.state === "done" || r.progress?.approxTotal != null);
  const total = rows.reduce((sum, r) => sum + (r.state === "done" ? r.project.cached : (r.progress?.approxTotal ?? 0)), 0);
  const share = known && total > 0 ? Math.min(1, fetched / total) : 0;
  return (
    <div className="first-sync">
      <h2 className="first-sync__title">Loading your Jira issues</h2>
      <p className="first-sync__text">The first sync fetches every issue once; afterwards only changes come in.</p>
      <div className="first-sync__bar" role="progressbar" aria-label="Issues loaded" aria-valuemin={0} aria-valuemax={known ? total : undefined} aria-valuenow={known ? fetched : undefined}>
        <div className="first-sync__bar-done" style={{ width: `${share * 100}%` }} />
      </div>
      <p className="first-sync__meta">{known ? `About ${fetched.toLocaleString("en")} of ${total.toLocaleString("en")} issues` : `${fetched.toLocaleString("en")} issues so far`}</p>
      <ul className="first-sync__spaces">
        {rows.map(({ project, state, progress: p }) => (
          <li key={project.id} className={`first-sync__space first-sync__space--${state}`}>
            <span className={`first-sync__icon first-sync__icon--${state}`} aria-hidden="true">
              {state === "done" ? "✓" : null}
            </span>
            <span className="first-sync__name">{project.name}</span>
            <span className="first-sync__detail">{state === "done" ? project.cached : state === "running" ? `page ${(p?.pagesDone ?? 0) + 1}` : "waiting"}</span>
          </li>
        ))}
      </ul>
      {rows.some((r) => r.state === "done") && <p className="first-sync__foot">You can already browse finished Spaces.</p>}
    </div>
  );
}

type SpaceState = "done" | "running" | "waiting";

function projectState(project: ProjectCounts, progress: SyncProgress | undefined): SpaceState {
  if (project.lastFullAt) return "done";
  return progress ? "running" : "waiting";
}
