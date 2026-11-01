import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { api, type ProjectCounts, type Settings } from "../../../api";
import { formatLocalTime } from "../../../lib/time";
import { SETTINGS_KEYS, useSettings } from "../../../queries/settings";
import { describeSyncState, useSyncOverview } from "../../../queries/sync";
import { fail, useStore } from "../../../state/store";
import { ConfirmSheet } from "../../ConfirmSheet";
import { Group, Row, Segmented, Switch } from "../controls";
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

const HISTORY_DAYS = ["30", "90", "180"] as const;

/** How long the changes you've seen stay cached; a value set elsewhere (a hand-edited file) presses none. */
function KeepHistory() {
  const client = useQueryClient();
  const { data: settings } = useSettings();
  if (!settings) return null;
  const save = (days: (typeof HISTORY_DAYS)[number]) =>
    void api.syncSave({ ...settings.sync, keepHistoryDays: Number(days) }).then(
      (saved) => client.setQueryData<Settings>(SETTINGS_KEYS.settings, (current) => current && { ...current, sync: saved }),
      (e: unknown) => fail(e, "Could not save the setting"),
    );
  const current = String(settings.sync.keepHistoryDays);
  return (
    <Segmented
      label="Keep history you've read"
      description="How long the changes on issues stay cached once you've seen them."
      options={HISTORY_DAYS}
      labels={{ "30": "30 days", "90": "90 days", "180": "180 days" }}
      value={HISTORY_DAYS.find((days) => days === current) ?? null}
      onChange={save}
    />
  );
}

/** Sync → Cache: the sync state with Sync Now, what the cache holds per tracked project, Reset Cache (after a
 *  confirmation), how long read history stays and clearing the cache on quit. */
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
      <KeepHistory />
      <ClearOnQuit />
      {confirming && <ConfirmSheet title="Reset the cache?" text={WHAT_STAYS} action="Reset Cache" onCancel={() => setConfirming(false)} onConfirm={reset} />}
    </Group>
  );
}
