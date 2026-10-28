import { X, Zap } from "lucide-react";
import { useIssue } from "../../queries/issues";

/** The view narrowed to one epic's issues ("Open as list view"): temporary like the Me chip, × clears it. */
export function EpicNarrowingChip({ epicId, onClear }: { epicId: string; onClear: () => void }) {
  const { data: epic } = useIssue(epicId);
  const name = epic ? `${epic.key} ${epic.summary}` : "epic";
  return (
    <button type="button" className="me-chip epic-narrowing" aria-label={`Only issues of ${name}, click to show all`} onClick={onClear}>
      <Zap size={12} strokeWidth={2.4} aria-hidden="true" />
      <span className="epic-narrowing__name">{epic?.summary ?? "Epic"}</span>
      <X size={12} strokeWidth={2.2} aria-hidden="true" />
    </button>
  );
}
