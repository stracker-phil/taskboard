import { useState } from "react";
import { api } from "../../../api";
import { useSyncOverview } from "../../../queries/sync";
import { fail, useStore } from "../../../state/store";
import { ConfirmSheet } from "../../ConfirmSheet";
import { Group, Row } from "../controls";
import { useBusyAction } from "../useBusyAction";
import { megabytes } from "../../../lib/size";
import { ShowDataFolder } from "./ShowDataFolder";

const KEPT = "Spaces, filter lists, views, bookmarks and what you've read";

/** Jira data is memory-only (ADR 0066). */
export function StorageGroup() {
  const { data: overview } = useSyncOverview();
  const [confirming, setConfirming] = useState(false);
  const [busy, run] = useBusyAction();

  const refetch = () => {
    setConfirming(false);
    void run(
      async () => {
        await api.cacheReset();
        useStore.getState().toast("success", "Taskboard fetches your issues again.");
      },
      (err) => fail(err, "Could not refetch from Jira"),
    );
  };

  return (
    <Group title="Jira data" anchor="cache" description="Nothing from Jira is written to disk; it is gone when Taskboard quits.">
      <Row label="Jira data in memory" description={overview ? `${megabytes(overview.cacheBytes)} while Taskboard runs.` : undefined}>
        <button type="button" className="btn" disabled={busy} onClick={() => setConfirming(true)}>
          Refetch from Jira
        </button>
      </Row>
      <Row label="Data folder" description={`Your setup lives here: ${KEPT}.`}>
        <ShowDataFolder />
      </Row>
      {confirming && (
        <ConfirmSheet
          title="Refetch from Jira?"
          text={`Taskboard drops the Jira data it holds and fetches your issues again. ${KEPT} stay.`}
          action="Refetch"
          onCancel={() => setConfirming(false)}
          onConfirm={refetch}
        />
      )}
    </Group>
  );
}
