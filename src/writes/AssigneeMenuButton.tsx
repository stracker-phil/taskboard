import type { ReactNode } from "react";
import type { IssueDetail } from "../api";
import { useShortcuts } from "../commands/useShortcuts";
import { useIssueActions } from "../views/list/useIssueActions";

/** The issue's assignee; while changes to Jira are on it opens the assignee menu. */
export function AssigneeMenuButton({ issue, children }: { issue: IssueDetail; children: ReactNode }) {
  const { assignTo } = useIssueActions();
  const { title } = useShortcuts();
  if (!assignTo) return <>{children}</>;
  return (
    <button
      type="button"
      className="assignee-menu-button"
      data-assign-menu={issue.id}
      aria-label={`Change the assignee of ${issue.key}`}
      title={title("Assign to…", "issue.assignTo")}
      aria-haspopup="menu"
      onClick={(e) => assignTo(issue, e.currentTarget)}
    >
      {children}
    </button>
  );
}
