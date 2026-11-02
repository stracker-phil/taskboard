import type { MouseEvent } from "react";
import type { Style } from "../api";
import type { MenuAt } from "../components/menu";

interface Issue {
  id: string;
  key: string;
  style: Style;
  statusName: string;
}

interface Props {
  issue: Issue;
  /** Null while changes to Jira are off: the square only names the status. */
  changeStatus: ((issue: Issue, at: MenuAt) => void) | null;
  /** Selects the row without opening it, as a status cell does. */
  onPick?: (id: string) => void;
}

const stop = (e: MouseEvent) => e.stopPropagation();

/** A row's or card's status square: its tooltip names the status, and a click opens the status menu. */
export function StatusSquareButton({ issue, changeStatus, onPick }: Props) {
  const className = `status-square status-square--${issue.style}`;
  if (!changeStatus) return <span className={className} title={issue.statusName} />;
  return (
    <button
      type="button"
      className={`${className} status-square--button`}
      data-status-menu={issue.id}
      aria-haspopup="menu"
      aria-label={`Change the status of ${issue.key} (${issue.statusName})`}
      title={issue.statusName}
      onClick={(e) => {
        stop(e);
        onPick?.(issue.id);
        changeStatus(issue, e.currentTarget);
      }}
      onDoubleClick={stop}
    />
  );
}
