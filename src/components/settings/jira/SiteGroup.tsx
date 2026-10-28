import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { api, type AppError, type ConnectionView, type TokenKind } from "../../../api";
import { SETTINGS_KEYS } from "../../../queries/settings";
import { fail } from "../../../state/store";
import { ControlRow, Group, Row, Segmented } from "../controls";
import { DraftField } from "../fields";
import { fieldErrorOf, useBusyAction, type FieldErrors, type ActionResult } from "../useBusyAction";

const TOKEN_PAGE = "https://id.atlassian.com/manage-profile/security/api-tokens";

const KIND_HINTS: Record<TokenKind, string> = {
  classic: "Calls your site directly.",
  scoped: "Calls api.atlassian.com; needs read:jira-work and read:jira-user.",
};

/** What the form edits; the token is kept apart and never shown once saved. */
interface Draft {
  siteUrl: string;
  email: string;
  tokenKind: TokenKind;
  cloudId: string;
}

function TokenHint({ hasToken }: { hasToken: boolean }) {
  return (
    <>
      {hasToken && "Saved in Keychain. Type a new one to replace it. "}
      <button type="button" className="link-btn" onClick={() => void api.openUrl(TOKEN_PAGE).catch((e) => fail(e, "Could not open the link"))}>
        Create a token at id.atlassian.com
      </button>
    </>
  );
}

/** Connection → Jira site: a small form that saves with Save or Test Connection (which saves first). */
export function SiteGroup({ view }: { view: ConnectionView }) {
  const client = useQueryClient();
  const saved = view.connection;
  const [edits, setEdits] = useState<Partial<Draft>>({});
  const [token, setToken] = useState("");
  const [errors, setErrors] = useState<FieldErrors>({});
  const [result, setResult] = useState<ActionResult | null>(null);
  // The site gave no cloud id: the owner types it in.
  const [askCloudId, setAskCloudId] = useState(false);
  const [busy, run] = useBusyAction();
  const draft: Draft = { siteUrl: saved.siteUrl, email: saved.email, tokenKind: saved.tokenKind, cloudId: saved.cloudId ?? "", ...edits };

  const edit = (patch: Partial<Draft>) => {
    setEdits((current) => ({ ...current, ...patch }));
    setResult(null);
  };

  const save = async () => {
    setErrors({});
    const cloudId = draft.tokenKind === "scoped" ? draft.cloudId.trim() || null : null;
    const input = { siteUrl: draft.siteUrl.trim(), email: draft.email.trim(), tokenKind: draft.tokenKind, tokenExpires: saved.tokenExpires, cloudId };
    const next = await api.connectionSave(input, token.trim() || null);
    client.setQueryData(SETTINGS_KEYS.connection, next);
    setEdits({});
    setToken("");
  };

  /** Field errors go under their field; anything else to the toast (Save) or the result line (Test). */
  const onError = (other: (err: AppError) => void) => (err: AppError) => {
    const fields = fieldErrorOf(err);
    if (fields) setErrors(fields);
    else other(err);
  };

  const onSave = () => void run(save, onError((err) => fail(err, "Could not save the connection")));

  const onTest = () =>
    void run(
      async () => {
        setResult(null);
        await save();
        const tested = await api.connectionTest();
        setAskCloudId(false);
        setResult({ ok: true, text: `Connected as ${tested.displayName} · ${tested.timeZone}` });
        void client.invalidateQueries({ queryKey: SETTINGS_KEYS.connection });
      },
      onError((err) => {
        if (err.kind === "not_found") setAskCloudId(true);
        setResult({ ok: false, text: err.message });
      }),
    );

  const cloudIdEditable = askCloudId || !saved.cloudId;
  return (
    <Group title="Jira site" anchor="site">
      <DraftField
        label="Site URL"
        wide
        placeholder="https://example.atlassian.net"
        autoComplete="url"
        value={draft.siteUrl}
        error={errors.siteUrl}
        onChange={(siteUrl) => edit({ siteUrl })}
      />
      <DraftField
        label="Email"
        type="email"
        wide
        placeholder="alex.morgan@example.com"
        autoComplete="email"
        value={draft.email}
        error={errors.email}
        onChange={(email) => edit({ email })}
      />
      <DraftField
        label="API token"
        type="password"
        wide
        autoComplete="off"
        placeholder={view.hasToken ? "Saved in Keychain" : "Paste your API token"}
        description={<TokenHint hasToken={view.hasToken} />}
        value={token}
        error={errors.token}
        onChange={(next) => {
          setToken(next);
          setResult(null);
        }}
      />
      <Segmented
        label="Token kind"
        description={KIND_HINTS[draft.tokenKind]}
        options={["classic", "scoped"] as const}
        value={draft.tokenKind}
        onChange={(tokenKind) => edit({ tokenKind })}
      />
      {draft.tokenKind === "scoped" &&
        (cloudIdEditable ? (
          <DraftField
            label="Cloud ID"
            mono
            wide
            description="Test Connection finds it; type it in when your site doesn't tell it."
            value={draft.cloudId}
            error={errors.cloudId}
            onChange={(cloudId) => edit({ cloudId })}
          />
        ) : (
          <Row label="Cloud ID" description="Found by Test Connection.">
            <span className="settings-value mono">{saved.cloudId}</span>
          </Row>
        ))}
      <ControlRow>
        <div className="settings-actions">
          <div className="button-row">
            <button type="button" className="btn" disabled={busy} onClick={onSave}>
              Save
            </button>
            <button type="button" className="btn btn-primary" disabled={busy} onClick={onTest}>
              Test Connection
            </button>
          </div>
          {result && (
            <p className={result.ok ? "settings-result ok" : "settings-result error"} role="status">
              {result.text}
            </p>
          )}
        </div>
      </ControlRow>
    </Group>
  );
}
