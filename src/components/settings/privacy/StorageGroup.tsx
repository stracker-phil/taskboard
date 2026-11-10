import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { api, type Settings, type SyncSettings } from "../../../api";
import { SETTINGS_KEYS, useSettings } from "../../../queries/settings";
import { useSyncOverview } from "../../../queries/sync";
import { fail, useStore } from "../../../state/store";
import { ConfirmSheet } from "../../ConfirmSheet";
import { Group, Row, Segmented, Switch } from "../controls";
import { useBusyAction } from "../useBusyAction";
import { ShowDataFolder } from "./ShowDataFolder";

const megabytes = (bytes: number) => `${(bytes / 1_000_000).toFixed(1)} MB`;

const WHAT_STAYS = "Spaces, filter lists, views, bookmarks and what you've read stay.";

/** What the data row, its sheet and its toasts say, per place the data lives. */
const COPY = {
  memory: {
    row: "Jira data in memory",
    describe: (size: string | undefined) => (size ? `${size} while Taskboard runs.` : undefined),
    button: "Refetch from Jira",
    danger: false,
    title: "Refetch from Jira?",
    text: `Taskboard drops the Jira data it holds and fetches your issues again. ${WHAT_STAYS}`,
    action: "Refetch",
    done: "Taskboard fetches your issues again.",
    failed: "Could not refetch from Jira",
  },
  disk: {
    row: "Copy of your Jira issues",
    describe: (size: string | undefined) => `${size ? `${size} on this Mac. ` : ""}Reset it when issues look out of date.`,
    button: "Reset Cache",
    danger: true,
    title: "Reset the cache?",
    text: `Taskboard deletes its copy of your Jira issues and fetches them again. ${WHAT_STAYS}`,
    action: "Reset Cache",
    done: "The cache is empty; the next sync fetches your issues again.",
    failed: "Could not reset the cache",
  },
};

/** Sync settings and a saver that resolves to whether it saved; null until the settings load. */
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

const HISTORY_DAYS = ["30", "90", "180"] as const;

/** How long the changes you've seen stay cached; a value set elsewhere (a hand-edited file) presses none. */
function KeepHistory({ sync, save }: { sync: SyncSettings; save: (change: Partial<SyncSettings>) => Promise<boolean> }) {
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

/** In memory mode nothing outlives the session, so the history and clear-on-quit settings are hidden. */
export function StorageGroup() {
  const saver = useSaveSync();
  const { data: overview } = useSyncOverview();
  const [confirming, setConfirming] = useState(false);
  const [busy, run] = useBusyAction();
  if (!saver) return null;
  const { sync, save } = saver;
  const inMemory = sync.memoryOnly;
  const copy = inMemory ? COPY.memory : COPY.disk;
  const toast = (text: string) => useStore.getState().toast("success", text);

  const switchMemory = (memoryOnly: boolean) =>
    void save({ memoryOnly }).then((saved) => {
      if (saved) toast("The cache is empty; Taskboard fetches your issues again.");
    });

  const resetCache = () => {
    setConfirming(false);
    void run(
      async () => {
        await api.cacheReset();
        toast(copy.done);
      },
      (err) => fail(err, copy.failed),
    );
  };

  return (
    <Group title="Jira data" anchor="cache">
      <Switch
        label="Keep Jira data in memory only"
        description="Nothing from Jira is written to disk. Each launch syncs again."
        checked={inMemory}
        onChange={switchMemory}
      />
      <Row label={copy.row} description={copy.describe(overview && megabytes(overview.cacheBytes))}>
        <button type="button" className={copy.danger ? "btn btn-danger" : "btn"} disabled={busy} onClick={() => setConfirming(true)}>
          {copy.button}
        </button>
      </Row>
      {!inMemory && <KeepHistory sync={sync} save={save} />}
      {!inMemory && (
        <Switch
          label="Clear the cache when Taskboard quits"
          description="No copy of your issues stays on this Mac; each launch fetches them again."
          checked={sync.clearCacheOnQuit}
          onChange={(clearCacheOnQuit) => void save({ clearCacheOnQuit })}
        />
      )}
      <Row label="Data folder" description="Your setup lives here; while the data is in memory, there is no cache.sqlite.">
        <ShowDataFolder />
      </Row>
      {confirming && (
        <ConfirmSheet title={copy.title} text={copy.text} action={copy.action} onCancel={() => setConfirming(false)} onConfirm={resetCache} />
      )}
    </Group>
  );
}
