import { useQuery, useQueryClient } from "@tanstack/react-query";
import { CircleAlert, CircleCheck, CircleX, Info, type LucideIcon } from "lucide-react";
import { api, type DiagnosticRow } from "../../../api";
import { SYNC_KEY } from "../../../queries/sync";
import { attempt } from "../../../state/requests";
import { Group, Row } from "../controls";

/** Under the sync's key, so a sync state change reads it again. */
const DIAGNOSTICS_KEY = [...SYNC_KEY, "diagnostics"] as const;

const TONES: Record<DiagnosticRow["tone"], { icon: LucideIcon; label: string }> = {
  ok: { icon: CircleCheck, label: "OK" },
  warn: { icon: CircleAlert, label: "Warning" },
  error: { icon: CircleX, label: "Error" },
  info: { icon: Info, label: "Info" },
};

function DiagnosticValue({ row, onRan }: { row: DiagnosticRow; onRan: () => void }) {
  const { icon: Icon, label } = TONES[row.tone];
  const { command, commandLabel } = row;
  const run = async (id: string) => {
    await attempt(api.runCommand(id), `Could not run ${commandLabel}`);
    onRan();
  };
  return (
    <>
      <span className="diagnostic-value" data-tone={row.tone}>
        <Icon size={14} role="img" aria-label={label} />
        <span className="settings-value">{row.value}</span>
      </span>
      {command && commandLabel && (
        <button type="button" className="btn" onClick={() => void run(command)}>
          {commandLabel}
        </button>
      )}
    </>
  );
}

/** Settings → Status: what the connection, the sync and this Mac report, read again when the window comes back. */
export function StatusPage() {
  const client = useQueryClient();
  const { data, error } = useQuery({ queryKey: DIAGNOSTICS_KEY, queryFn: api.diagnosticsGet, refetchOnWindowFocus: "always" });
  const reload = () => void client.invalidateQueries({ queryKey: DIAGNOSTICS_KEY });
  if (error && !data) return <p className="settings-empty" role="alert">{String((error as { message?: string }).message ?? error)}</p>;
  return data?.groups.map((group) => (
    <Group key={group.title} title={group.title} anchor={group.title === "Jira" ? "status" : undefined}>
      {group.rows.map((row) => (
        <Row key={row.label} label={row.label}>
          <DiagnosticValue row={row} onRan={reload} />
        </Row>
      ))}
    </Group>
  ));
}
