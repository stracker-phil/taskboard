import { useState } from "react";
import { api, type ProjectCounts } from "../../../api";
import { formatLocalTime } from "../../../lib/time";
import { describeSyncState, useSyncOverview } from "../../../queries/sync";
import { fail, useStore } from "../../../state/store";
import { ConfirmSheet } from "../../ConfirmSheet";
import { Group, Row } from "../controls";
import { useBusyAction } from "../useBusyAction";

const WHAT_STAYS = "Taskboard deletes its copy of your Jira issues and fetches them again. Spaces, filter lists, views and pins stay.";

const issues = (n: number) => (n === 1 ? "1 issue" : `${n} issues`);

/** A tracked project: its key and last full sync on the left, what the cache holds of it on the right. */
function ProjectRow({ project }: { project: ProjectCounts }) {
  const synced = project.lastFullAt ? `Synced at ${formatLocalTime(project.lastFullAt)}` : "Not synced yet";
  return (
    <Row label={project.name} description={<><span className="mono">{project.key}</span> · {synced}</>}>
      <span className="settings-value">
        {issues(project.cached)} · {project.open} open
      </span>
    </Row>
  );
}

/** Sync → Cache: the sync state with Sync Now, what the cache holds per tracked project, and Reset Cache
 *  (after a confirmation). */
export function CacheGroup() {
  const [confirming, setConfirming] = useState(false);
  const [busy, run] = useBusyAction();
  const { data: overview } = useSyncOverview();
  const running = overview?.state.state === "running";

  const syncNow = () => void api.syncRun().catch((e: unknown) => fail(e, "Could not start the sync"));

  const reset = () => {
    setConfirming(false);
    void run(
      async () => {
        await api.cacheReset();
        useStore.getState().toast("success", "The cache is empty; the next sync fetches your issues again.");
      },
      (err) => fail(err, "Could not reset the cache"),
    );
  };

  return (
    <Group title="Cache" anchor="cache">
      <Row label="Sync" description={overview ? describeSyncState(overview.state) : undefined}>
        <button type="button" className="btn" disabled={!overview || running} onClick={syncNow}>
          Sync Now
        </button>
      </Row>
      {overview?.projects.length === 0 ? (
        <div className="settings-row">
          <span className="settings-empty">Track projects on the Connection page.</span>
        </div>
      ) : (
        overview?.projects.map((project) => <ProjectRow key={project.id} project={project} />)
      )}
      <Row label="Copy of your Jira issues" description="Reset it when issues look out of date.">
        <button type="button" className="btn btn-danger" disabled={busy} onClick={() => setConfirming(true)}>
          Reset Cache
        </button>
      </Row>
      {confirming && <ConfirmSheet title="Reset the cache?" text={WHAT_STAYS} action="Reset Cache" onCancel={() => setConfirming(false)} onConfirm={reset} />}
    </Group>
  );
}
