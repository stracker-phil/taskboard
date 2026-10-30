import type { IssueDetail } from "../api";
import { StatusPill, type PillVariant } from "../components/issue/StatusPill";
import { useIssueActions } from "../views/list/useIssueActions";

/** The issue's status pill; while changes to Jira are on it opens the status menu (also S). */
export function StatusMenuButton({ issue, variant }: { issue: IssueDetail; variant?: PillVariant }) {
  const { changeStatus } = useIssueActions();
  const pill = <StatusPill style={issue.style} name={issue.statusName} variant={variant} />;
  if (!changeStatus) return pill;
  return (
    <button
      type="button"
      className="status-menu-button"
      data-status-menu={issue.id}
      aria-label={`Change the status of ${issue.key}`}
      title="Change status (S)"
      aria-haspopup="menu"
      onClick={(e) => changeStatus(issue, e.currentTarget)}
    >
      {pill}
    </button>
  );
}
