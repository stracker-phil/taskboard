import type { MouseEvent } from "react";

const keepToItself = (e: MouseEvent) => e.stopPropagation();

/** A row's or card's checkbox for picking it into a bulk change; its clicks never reach the row. */
export function PickBox({ issueKey, picked, onToggle, className }: { issueKey: string; picked: boolean; onToggle: () => void; className: string }) {
  return (
    <input
      type="checkbox"
      className={`pick-box ${className}`}
      checked={picked}
      aria-label={`Pick ${issueKey}`}
      onChange={onToggle}
      onClick={keepToItself}
      onDoubleClick={keepToItself}
    />
  );
}
