import { useQuery, useQueryClient } from "@tanstack/react-query";
import { CircleAlert, CircleCheck, CircleX, Copy, type LucideIcon } from "lucide-react";
import { api, type DiagnosticRow } from "../../../api";
import { copyWithToast } from "../../../lib/clipboard";
import { SYNC_KEY } from "../../../queries/sync";
import { attempt } from "../../../state/requests";
import { Group, Row } from "../controls";

/** Under the sync's key, so a sync state change reads it again. */
const DIAGNOSTICS_KEY = [...SYNC_KEY, "diagnostics"] as const;

/** Only tones that tell something get an icon; an info value has none. */
const TONES: Partial<Record<DiagnosticRow["tone"], { icon: LucideIcon; label: string }>> = {
  ok: { icon: CircleCheck, label: "OK" },
  warn: { icon: CircleAlert, label: "Warning" },
  error: { icon: CircleX, label: "Error" },
};

/** A click copies the value; the clipboard icon shows on hover, in the tone icon's place. */
function DiagnosticValue({ row, onRan }: { row: DiagnosticRow; onRan: () => void }) {
  const tone = TONES[row.tone];
  const { command, commandLabel } = row;
  const run = async (id: string) => {
    await attempt(api.runCommand(id), `Could not run ${commandLabel}`);
    onRan();
  };
  return (
    <>
      <button
        type="button"
        className="diagnostic-value"
        data-tone={row.tone}
        title="Click to copy"
        onClick={() => copyWithToast(row.copyText ?? row.value, `Copied ${row.label}`)}
      >
        {tone && <tone.icon className="diagnostic-value__tone" size={14} role="img" aria-label={tone.label} />}
        <Copy className="diagnostic-value__copy" size={14} aria-hidden="true" />
        <span className="settings-value">{row.value}</span>
      </button>
      {command && commandLabel && (
        <button type="button" className="btn" onClick={() => void run(command)}>
          {commandLabel}
        </button>
      )}
    </>
  );
}

/** The groups `diagnostics_get` reports, as the search finds them; their rows are values, not settings. */
export const STATUS_GROUPS = [
  { id: "status", title: "Jira" },
  { id: "status-sync", title: "Sync" },
  { id: "status-mac", title: "This Mac" },
  { id: "status-sent", title: "Changes Sent to Jira" },
];

/** Settings → Status: what the connection, the sync and this Mac report, read again when the window comes back. */
export function StatusPage() {
  const client = useQueryClient();
  const { data, error } = useQuery({ queryKey: DIAGNOSTICS_KEY, queryFn: api.diagnosticsGet, refetchOnWindowFocus: "always" });
  const reload = () => void client.invalidateQueries({ queryKey: DIAGNOSTICS_KEY });
  if (error && !data) return <p className="settings-empty" role="alert">{String((error as { message?: string }).message ?? error)}</p>;
  return data?.groups.map((group) => (
    <Group key={group.title} title={group.title} anchor={STATUS_GROUPS.find((known) => known.title === group.title)?.id}>
      {group.rows.map((row, index) => (
        <Row key={`${index}:${row.label}`} label={row.label}>
          <DiagnosticValue row={row} onRan={reload} />
        </Row>
      ))}
    </Group>
  ));
}
