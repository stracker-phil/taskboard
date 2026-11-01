import type { IssueDetail } from "../api";
import { useShortcuts } from "../commands/useShortcuts";
import { StatusPill, type PillVariant } from "../components/issue/StatusPill";
import { useIssueActions } from "../views/list/useIssueActions";

/** The issue's status pill; while changes to Jira are on it opens the status menu. */
export function StatusMenuButton({ issue, variant }: { issue: IssueDetail; variant?: PillVariant }) {
  const { changeStatus } = useIssueActions();
  const { title } = useShortcuts();
  const pill = <StatusPill style={issue.style} name={issue.statusName} variant={variant} />;
  if (!changeStatus) return pill;
  return (
    <button
      type="button"
      className="status-menu-button"
      data-status-menu={issue.id}
      aria-label={`Change the status of ${issue.key}`}
      title={title("Change status", "issue.changeStatus")}
      aria-haspopup="menu"
      onClick={(e) => changeStatus(issue, e.currentTarget)}
    >
      {pill}
    </button>
  );
}
