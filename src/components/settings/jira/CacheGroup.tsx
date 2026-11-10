import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { api, type ProjectCounts, type Settings, type SyncSettings } from "../../../api";
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

/** The sync settings once loaded, and `save`, which applies one change and resolves to whether it saved. */
function useSaveSync() {
  const client = useQueryClient();
  const { data: settings } = useSettings();
  if (!settings) return null;
  const save = (change: Partial<SyncSettings>) =>
    api.syncSave({ ...settings.sync, ...change }).then(
      (saved) => {
        client.setQueryData<Settings>(SETTINGS_KEYS.settings, (current) => current && { ...current, sync: saved });
        return true;
      },
      (e: unknown) => {
        fail(e, "Could not save the setting");
        return false;
      },
    );
  return { sync: settings.sync, save };
}

function MemoryOnly() {
  const saver = useSaveSync();
  if (!saver) return null;
  const { sync, save } = saver;
  const change = (memoryOnly: boolean) =>
    void save({ memoryOnly }).then((saved) => {
      if (saved) useStore.getState().toast("success", "The cache is empty; Taskboard fetches your issues again.");
    });
  return (
    <Switch
      label="Keep Jira data in memory only"
      description="Nothing from Jira is written to disk. Each launch syncs again."
      checked={sync.memoryOnly}
      onChange={change}
    />
  );
}

/** Empties the cache whenever Taskboard quits, so no Jira data stays on the Mac between sessions. */
function ClearOnQuit() {
  const saver = useSaveSync();
  if (!saver || saver.sync.memoryOnly) return null;
  const { sync, save } = saver;
  return (
    <Switch
      label="Clear the cache when Taskboard quits"
      description="No copy of your issues stays on this Mac; each launch fetches them again."
      checked={sync.clearCacheOnQuit}
      onChange={(clearCacheOnQuit) => void save({ clearCacheOnQuit })}
    />
  );
}

/** The tracked projects. Their keys come from the cache, so an emptied cache lists none until the sync stores
 *  them again; the last rows stay meanwhile, so the controls below don't move. */
function useSteadyProjects(projects: ProjectCounts[] | undefined) {
  const [kept, setKept] = useState<ProjectCounts[]>([]);
  if (projects?.length && projects !== kept) setKept(projects);
  return projects?.length === 0 && kept.length > 0 ? kept : projects;
}

const HISTORY_DAYS = ["30", "90", "180"] as const;

/** How long the changes you've seen stay cached; a value set elsewhere (a hand-edited file) presses none. */
function KeepHistory() {
  const saver = useSaveSync();
  if (!saver) return null;
  const { sync, save } = saver;
  const current = String(sync.keepHistoryDays);
  return (
    <Segmented
      label="Keep history you've read"
      description="How long the changes on issues stay cached once you've seen them."
      options={HISTORY_DAYS}
      labels={{ "30": "30 days", "90": "90 days", "180": "180 days" }}
      value={HISTORY_DAYS.find((days) => days === current) ?? null}
      onChange={(days) => void save({ keepHistoryDays: Number(days) })}
    />
  );
}

/** Sync → Cache: the sync state, the tracked projects and how the cache is kept. */
export function CacheGroup() {
  const [confirming, setConfirming] = useState(false);
  const [busy, run] = useBusyAction();
  const { data: overview } = useSyncOverview();
  const running = overview?.state.state === "running";
  const projects = useSteadyProjects(overview?.projects);

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
      {projects?.length === 0 ? (
        <div className="settings-row">
          <span className="settings-empty">Track projects on the Connection page.</span>
        </div>
      ) : (
        projects?.map((project) => <ProjectRow key={project.id} project={project} />)
      )}
      <Row label="Copy of your Jira issues" description="Reset it when issues look out of date.">
        <button type="button" className="btn btn-danger" disabled={busy} onClick={() => setConfirming(true)}>
          Reset Cache
        </button>
      </Row>
      <KeepHistory />
      <MemoryOnly />
      <ClearOnQuit />
      {confirming && <ConfirmSheet title="Reset the cache?" text={WHAT_STAYS} action="Reset Cache" onCancel={() => setConfirming(false)} onConfirm={reset} />}
    </Group>
  );
}
