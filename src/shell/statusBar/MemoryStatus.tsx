import { MemoryStick } from "lucide-react";
import { megabytes } from "../../lib/size";
import { useSettings } from "../../queries/settings";
import { useSyncOverview } from "../../queries/sync";

const COUNT = new Intl.NumberFormat();

/** A memory chip while the Jira data lives in memory only (ADR 0057); its tooltip says how much it holds. Its slot
 *  stays when it is off, so turning memory mode on or off moves nothing beside it. */
export function MemoryStatus() {
  const { data: settings } = useSettings();
  const { data: overview } = useSyncOverview();
  const on = settings?.sync.memoryOnly ?? false;
  const issues = overview?.projects.reduce((sum, p) => sum + p.cached, 0) ?? 0;
  const detail = overview
    ? `Jira data is kept in memory only\n${COUNT.format(issues)} ${issues === 1 ? "issue" : "issues"} · ${megabytes(overview.cacheBytes)}\nGone when Taskboard quits; the next launch syncs again`
    : "Jira data is kept in memory only";
  return (
    <span className="status-memory" data-on={on} aria-hidden={!on}>
      {on && (
        <span role="img" aria-label="Jira data in memory" title={detail}>
          <MemoryStick size={14} strokeWidth={1.8} aria-hidden="true" />
        </span>
      )}
    </span>
  );
}
