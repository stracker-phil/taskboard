import type { CSSProperties } from "react";
import { ChevronDown } from "lucide-react";

/** A row's disclosure in Hierarchy: collapses or expands its subtree without selecting the row. */
interface Props {
  open: boolean;
  onToggle: () => void;
  className?: string;
  style?: CSSProperties;
}

export function SubtaskCaret({ open, onToggle, className, style }: Props) {
  return (
    <button
      type="button"
      className={className ? `subtask-caret ${className}` : "subtask-caret"}
      style={style}
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
