import type { EpicChip as Epic } from "../../api";

export function EpicChip({ epic }: { epic: Epic }) {
  return (
    <span className={`epic-chip epic-chip--${epic.color}`} title={epic.name} data-issue-id={epic.id}>
      <span className="epic-chip__name">{epic.name}</span>
    </span>
  );
}
