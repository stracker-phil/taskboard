import { useId, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { api, type AppError } from "../../../api";
import { SETTINGS_KEYS, useScope, useSettings } from "../../../queries/settings";
import { fail } from "../../../state/store";
import { ControlRow, Group, Row } from "../controls";
import { FieldError } from "../fields";
import { useBusyAction, type ActionResult } from "../useBusyAction";

/** Sync → Scope: the JQL that decides which issues Taskboard mirrors; the default follows the tracked projects. */
export function ScopeGroup() {
  const client = useQueryClient();
  const id = useId();
  const resultId = `${id}-result`;
  const { data: scope } = useScope();
  const { data: settings } = useSettings();
  const [draft, setDraft] = useState<string | null>(null);
  const [result, setResult] = useState<ActionResult | null>(null);
  const [busy, run] = useBusyAction();
  const saved = scope?.jql ?? "";
  const text = draft ?? saved;

  /** Jira's messages go under the field; anything else to a toast. */
  const onError = (failure: string) => (err: AppError) => {
    if (err.kind === "invalid_jql") setResult({ ok: false, text: err.message });
    else fail(err, failure);
  };

  const validate = () =>
    void run(
      async () => {
        setResult(null);
        await api.scopeValidate(text);
        setResult({ ok: true, text: "Jira accepts this query." });
      },
      onError("Could not validate the query"),
    );

  const save = (jql: string | null) =>
    void run(
      async () => {
        setResult(null);
        client.setQueryData(SETTINGS_KEYS.scope, await api.scopeSave(jql));
        setDraft(null);
      },
      onError("Could not save the scope"),
    );

  const keepDays = settings?.sync.keepDoneDays;
  const hint = scope?.isDefault && keepDays !== undefined ? `Default: open work plus done issues of the last ${keepDays} days in tracked projects.` : undefined;
  return (
    <Group title="Scope" anchor="scope" description="The issues Taskboard copies from Jira.">
      <Row label="Query" htmlFor={id} description={hint} stacked>
        <textarea
          id={id}
          className="input textarea mono"
          rows={4}
          spellCheck={false}
          placeholder="Track a project on the Connection page, or type a query."
          aria-invalid={result && !result.ok ? true : undefined}
          aria-describedby={result ? resultId : undefined}
          value={text}
          onChange={(e) => {
            setDraft(e.target.value);
            setResult(null);
          }}
        />
        {result?.ok ? (
          <span id={resultId} className="settings-result ok" role="status">
            {result.text}
          </span>
        ) : (
          <FieldError id={resultId} message={result?.text} />
        )}
      </Row>
      <ControlRow>
        <div className="button-row">
          <button type="button" className="btn" disabled={busy || !text.trim()} onClick={validate}>
            Validate
          </button>
          <button type="button" className="btn btn-primary" disabled={busy || draft === null || draft === saved} onClick={() => save(text)}>
            Save
          </button>
          {scope && !scope.isDefault && (
            <button type="button" className="btn" disabled={busy} onClick={() => save(null)}>
              Reset to Default
            </button>
          )}
        </div>
      </ControlRow>
    </Group>
  );
}
