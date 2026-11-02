import type { MouseEvent } from "react";
import { useShortcuts } from "../../commands/useShortcuts";

const keepToItself = (e: MouseEvent) => e.stopPropagation();

/** A row's or card's checkbox for picking it into a bulk change; its clicks never reach the row. */
export function PickBox({ issueKey, picked, onToggle, className }: { issueKey: string; picked: boolean; onToggle: () => void; className?: string }) {
  const { title } = useShortcuts();
  return (
    <input
      type="checkbox"
      className={className ? `pick-box ${className}` : "pick-box"}
      checked={picked}
      aria-label={`Pick ${issueKey}`}
      title={title(picked ? "Drop from the picks" : "Pick", "issue.togglePick")}
      onChange={onToggle}
      onClick={keepToItself}
      onDoubleClick={keepToItself}
    />
  );
}
