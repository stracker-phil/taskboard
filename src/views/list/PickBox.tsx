import { type MouseEvent, useEffect, useRef } from "react";
import { useShortcuts } from "../../commands/useShortcuts";
import type { GroupPicks } from "./picking";

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

/** A group's or column's checkbox: ticked when all its issues are picked, a dash when some are. It picks or
 *  unpicks them all; its clicks never reach the header, so the group doesn't collapse. `group` is null for the
 *  one box of an ungrouped view. */
export function GroupBox({ group, picks, onToggle, className }: { group: string | null; picks: GroupPicks; onToggle: () => void; className?: string }) {
  const scope = group === null ? "all issues" : `all in ${group}`;
  const ref = useRef<HTMLInputElement>(null);
  // `indeterminate` is a property only; a click clears it, so every render sets it again.
  useEffect(() => {
    if (ref.current) ref.current.indeterminate = picks === "some";
  });
  return (
    <input
      ref={ref}
      type="checkbox"
      className={className ? `pick-box group-box ${className}` : "pick-box group-box"}
      checked={picks === "all"}
      aria-label={`Pick ${scope}`}
      title={picks === "all" ? `Drop ${scope} from the picks` : `Pick ${scope}`}
      onChange={onToggle}
      onClick={keepToItself}
      onDoubleClick={keepToItself}
    />
  );
}
