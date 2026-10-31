import type { ReactNode } from "react";
import type { IssueDetail } from "../api";
import { useIssueActions } from "../views/list/useIssueActions";

/** The issue's assignee; while changes to Jira are on it opens the assignee menu (also A). */
export function AssigneeMenuButton({ issue, children }: { issue: IssueDetail; children: ReactNode }) {
  const { assignTo } = useIssueActions();
  if (!assignTo) return <>{children}</>;
  return (
    <button
      type="button"
      className="assignee-menu-button"
      data-assign-menu={issue.id}
      aria-label={`Change the assignee of ${issue.key}`}
      title="Assign to… (A)"
      aria-haspopup="menu"
      onClick={(e) => assignTo(issue, e.currentTarget)}
    >
      {children}
    </button>
  );
}
