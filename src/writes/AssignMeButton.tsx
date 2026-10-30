import { UserRound } from "lucide-react";
import type { IssueDetail } from "../api";
import { useIssueActions } from "../views/list/useIssueActions";

/** Hidden while changes to Jira are off. */
export function AssignMeButton({ issue, className }: { issue: IssueDetail; className: string }) {
  const { toggleMine } = useIssueActions();
  if (!toggleMine) return null;
  const mine = issue.assignedToMe;
  return (
    <button
      type="button"
      className={className}
      aria-label={mine ? `Unassign ${issue.key}` : `Assign ${issue.key} to me`}
      title={`${mine ? "Unassign" : "Assign to me"} (I)`}
      aria-pressed={mine}
      onClick={() => toggleMine(issue.id)}
    >
      <UserRound size={14} strokeWidth={2} aria-hidden="true" />
    </button>
  );
}
