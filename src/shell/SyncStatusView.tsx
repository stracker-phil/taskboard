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
      return <Loading projects={projects} progress={progress} />;
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

function Loading({ projects, progress }: { projects: ProjectCounts[]; progress: Record<string, SyncProgress> }) {
  return (
    <div className="sync-status">
      <p>Loading your Jira issues</p>
      <ul className="sync-status__projects">
        {projects.map((project) => (
          <ProjectProgress key={project.id} project={project} progress={progress[project.id]} />
        ))}
      </ul>
    </div>
  );
}

function ProjectProgress({ project, progress }: { project: ProjectCounts; progress?: SyncProgress }) {
  const fetched = progress?.fetched ?? 0;
  const total = progress?.approxTotal ?? null;
  const share = total ? Math.min(1, fetched / total) : 0;
  const label = !progress ? "Waiting" : total === null ? `${fetched} fetched` : `${fetched} of ${total}`;
  return (
    <li className="sync-status__project">
      <span>{project.name}</span>
      <span className="sync-status__detail">{label}</span>
      <div
        className="sync-status__bar"
        role="progressbar"
        aria-label={project.name}
        aria-valuemin={0}
        aria-valuemax={total ?? undefined}
        aria-valuenow={total === null ? undefined : fetched}
      >
        <div className="sync-status__bar-done" style={{ width: `${share * 100}%` }} />
      </div>
    </li>
  );
}
