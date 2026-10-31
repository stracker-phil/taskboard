import { api, type Reverse, type WriteOutcome } from "../api";
import { attempt } from "../state/requests";
import { useStore } from "../state/store";

/** Awaits a Jira change and toasts it with a "Change back" action, which is a new change and never an undo. */
export function runWrite(issueId: string, call: Promise<WriteOutcome>, failure: string) {
  void attempt(call, failure).then((outcome) => {
    if (outcome) showOutcome(issueId, outcome);
  });
}

/** The toast for a change Jira took, with its "Change back" action when there is one. */
export function showOutcome(issueId: string, outcome: WriteOutcome) {
  const action = outcome.reverse ? changeBackAction(issueId, outcome.reverse) : undefined;
  useStore.getState().toast("success", outcome.done, action);
}

function changeBackAction(issueId: string, reverse: Reverse) {
  return { label: reverse.label, run: () => runWrite(issueId, api.issueWrite(issueId, reverse.change), "Could not change it back") };
}

export function explainWritesOff() {
  useStore.getState().toast("info", "Changes to Jira are off. Turn them on in Settings → Connection.");
}
