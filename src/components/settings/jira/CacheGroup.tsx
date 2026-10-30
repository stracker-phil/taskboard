import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { api, type ProjectCounts, type Settings } from "../../../api";
import { formatLocalTime } from "../../../lib/time";
import { SETTINGS_KEYS, useSettings } from "../../../queries/settings";
import { describeSyncState, useSyncOverview } from "../../../queries/sync";
import { fail, useStore } from "../../../state/store";
import { ConfirmSheet } from "../../ConfirmSheet";
import { Group, Row, Switch } from "../controls";
import { useBusyAction } from "../useBusyAction";

const WHAT_STAYS = "Taskboard deletes its copy of your Jira issues and fetches them again. Spaces, filter lists, views, bookmarks and what you've read stay.";

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

/** Empties the cache whenever Taskboard quits, so no Jira data stays on the Mac between sessions. */
function ClearOnQuit() {
  const client = useQueryClient();
  const { data: settings } = useSettings();
  if (!settings) return null;
  const save = (clearCacheOnQuit: boolean) =>
    void api.syncSave({ ...settings.sync, clearCacheOnQuit }).then(
      (saved) => client.setQueryData<Settings>(SETTINGS_KEYS.settings, (current) => current && { ...current, sync: saved }),
      (e: unknown) => fail(e, "Could not save the setting"),
    );
  return (
    <Switch
      label="Clear the cache when Taskboard quits"
      description="No copy of your issues stays on this Mac; each launch fetches them again."
      checked={settings.sync.clearCacheOnQuit}
      onChange={save}
    />
  );
}

/** Sync → Cache: the sync state with Sync Now, what the cache holds per tracked project, Reset Cache (after a
 *  confirmation) and clearing it on quit. */
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
      <ClearOnQuit />
      {confirming && <ConfirmSheet title="Reset the cache?" text={WHAT_STAYS} action="Reset Cache" onCancel={() => setConfirming(false)} onConfirm={reset} />}
    </Group>
  );
}
