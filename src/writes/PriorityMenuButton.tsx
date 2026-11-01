import type { ReactNode } from "react";
import type { IssueDetail } from "../api";
import { useShortcuts } from "../commands/useShortcuts";
import { useIssueActions } from "../views/list/useIssueActions";

/** The issue's priority; while changes to Jira are on it opens the priority menu. */
export function PriorityMenuButton({ issue, children }: { issue: IssueDetail; children: ReactNode }) {
  const { changePriority } = useIssueActions();
  const { title } = useShortcuts();
  if (!changePriority) return <>{children}</>;
  return (
    <button
      type="button"
      className="field-menu-button"
      data-priority-menu={issue.id}
      aria-label={`Change the priority of ${issue.key}`}
      title={title("Set priority…", "issue.changePriority")}
      aria-haspopup="menu"
      onClick={(e) => changePriority(issue, e.currentTarget)}
    >
      {children}
    </button>
  );
}
