import { ChevronDown } from "lucide-react";

/** A parent's disclosure in Hierarchy: collapses or expands its subtasks without selecting the row. */
export function SubtaskCaret({ open, onToggle }: { open: boolean; onToggle: () => void }) {
  return (
    <button
      type="button"
      className="subtask-caret"
      aria-label={open ? "Collapse subtasks" : "Expand subtasks"}
      aria-expanded={open}
      onClick={(e) => {
        e.stopPropagation();
        onToggle();
      }}
      onDoubleClick={(e) => e.stopPropagation()}
    >
      <ChevronDown size={12} strokeWidth={2.2} aria-hidden="true" />
    </button>
  );
}
