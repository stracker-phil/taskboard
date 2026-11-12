import { useState } from "react";
import { api, type ProjectCounts } from "../../../api";
import { formatLocalTime } from "../../../lib/time";
import { useLocalDays } from "../../../queries/localDays";
import { describeSyncState, useSyncOverview } from "../../../queries/sync";
import { fail } from "../../../state/store";
import { Group, Row } from "../controls";

const issues = (n: number) => (n === 1 ? "1 issue" : `${n} issues`);

/** One tracked project and what the cache holds of it. */
function ProjectRow({ project }: { project: ProjectCounts }) {
  const days = useLocalDays();
  const synced = project.lastFullAt ? `Synced at ${formatLocalTime(project.lastFullAt, days)}` : "Not synced yet";
  return (
    <Row label={project.name} description={<><span className="mono">{project.key}</span> · {synced}</>}>
      <span className="settings-value">
        {issues(project.cached)} · {project.open} open
      </span>
    </Row>
  );
}

/** The tracked projects. Their keys come from the cache, so an emptied cache lists none until the sync stores
 *  them again; the last rows stay meanwhile, so the page doesn't jump. */
function useSteadyProjects(projects: ProjectCounts[] | undefined) {
  const [lastNonEmpty, setLastNonEmpty] = useState<ProjectCounts[]>([]);
  if (projects?.length && projects !== lastNonEmpty) setLastNonEmpty(projects);
  return projects?.length === 0 && lastNonEmpty.length > 0 ? lastNonEmpty : projects;
}

/** Sync → Synced: the sync state with Sync Now and what Taskboard holds per tracked project. */
export function SyncedGroup() {
  const { data: overview } = useSyncOverview();
  const days = useLocalDays();
  const running = overview?.state.state === "running";
  const projects = useSteadyProjects(overview?.projects);
  const syncNow = () => void api.syncRun().catch((e: unknown) => fail(e, "Could not start the sync"));

  return (
    <Group title="Synced" anchor="synced">
      <Row label="Sync" anchor="sync-state" description={overview ? describeSyncState(overview.state, days) : undefined}>
        <button type="button" className="btn" disabled={!overview || running} onClick={syncNow}>
          Sync Now
        </button>
      </Row>
      {projects?.length === 0 ? (
        <div className="settings-row">
          <span className="settings-empty">Track projects on the Connection page.</span>
        </div>
      ) : (
        projects?.map((project) => <ProjectRow key={project.id} project={project} />)
      )}
    </Group>
  );
}
