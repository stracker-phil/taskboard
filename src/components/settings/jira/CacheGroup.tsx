import { useId, useState } from "react";
import { api } from "../../../api";
import { fail, useStore } from "../../../state/store";
import { Modal } from "../../Modal";
import { Group, Row } from "../controls";
import { useBusyAction } from "../useBusyAction";

const WHAT_STAYS = "Taskboard deletes its copy of your Jira issues and fetches them again. Spaces, filter lists, views and pins stay.";

function ConfirmReset({ onCancel, onConfirm }: { onCancel: () => void; onConfirm: () => void }) {
  const id = useId();
  return (
    <Modal role="alertdialog" labelledBy={`${id}-title`} describedBy={`${id}-text`} onClose={onCancel}>
      <h2 id={`${id}-title`}>Reset the cache?</h2>
      <p id={`${id}-text`}>{WHAT_STAYS}</p>
      <div className="actions">
        <button type="button" className="btn" onClick={onCancel}>
          Cancel
        </button>
        <button type="button" className="btn btn-danger" onClick={onConfirm}>
          Reset Cache
        </button>
      </div>
    </Modal>
  );
}

/** Sync → Cache: deletes the copy of the Jira issues after a confirmation. */
export function CacheGroup() {
  const [confirming, setConfirming] = useState(false);
  const [busy, run] = useBusyAction();

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
      <Row label="Copy of your Jira issues" description="Reset it when issues look out of date.">
        <button type="button" className="btn btn-danger" disabled={busy} onClick={() => setConfirming(true)}>
          Reset Cache
        </button>
      </Row>
      {confirming && <ConfirmReset onCancel={() => setConfirming(false)} onConfirm={reset} />}
    </Group>
  );
}
