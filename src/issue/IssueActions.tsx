import { CheckCheck, CircleDot, Copy, ExternalLink, type LucideIcon, Maximize2, X } from "lucide-react";
import type { ReactNode } from "react";
import { api, type IssueDetail } from "../api";
import { useShortcuts } from "../commands/useShortcuts";
import { PinButton } from "../pins/PinButton";
import { attempt } from "../state/requests";
import { useIssueActions } from "../views/list/useIssueActions";
import { holdUnread } from "./useSeenAfter";

/** Read marks it read now; unread keeps it unread while it stays open. */
export function toggleRead(issue: IssueDetail) {
  if (issue.unread) {
    void attempt(api.issuesMarkSeen([issue.id]), "Could not mark it read");
    return;
  }
  holdUnread(issue.id);
  void attempt(api.issueMarkUnread(issue.id), "Could not mark it unread");
}

/** A header action; its command's key shows in the tooltip. */
export function ActionButton({
  label,
  command,
  icon: Icon,
  onClick,
  disabled,
  className = "issue-actions__btn",
  iconSize = 14,
}: {
  label: string;
  command: string;
  icon: LucideIcon;
  onClick: () => void;
  disabled?: boolean;
  className?: string;
  iconSize?: number;
}) {
  const { title } = useShortcuts();
  return (
    <button type="button" className={className} aria-label={label} title={title(label, command)} disabled={disabled} onClick={onClick}>
      <Icon size={iconSize} strokeWidth={2} aria-hidden="true" />
    </button>
  );
}

/** Header actions shared by the preview and the Issue view; dimmed until hovered, Close always full. */
export function IssueActions({
  issue,
  onClose,
  closeCommand,
  onExpand,
  before,
}: {
  issue: IssueDetail | undefined;
  onClose: () => void;
  closeCommand: string;
  onExpand?: () => void;
  before?: ReactNode;
}) {
  const actions = useIssueActions();
  const openInJira = actions.openInJira;
  return (
    <div className="issue-actions">
      {before}
      {issue && (
        <>
          <ActionButton
            label={issue.unread ? "Mark as read" : "Mark as unread"}
            command="issue.markSeen"
            icon={issue.unread ? CheckCheck : CircleDot}
            onClick={() => toggleRead(issue)}
          />
          <PinButton kind="issue" target={issue.id} label="Bookmark this issue" meta={issue.key} command="issue.togglePin" className="issue-actions__btn" />
          <ActionButton label="Copy as Markdown" command="issue.copyMarkdown" icon={Copy} onClick={() => actions.copyMarkdown(issue)} />
          {openInJira && <ActionButton label="Open in Jira" command="issue.openInJira" icon={ExternalLink} onClick={() => openInJira(issue.key)} />}
        </>
      )}
      {onExpand && <ActionButton label="Full view" command="issue.open" icon={Maximize2} onClick={onExpand} />}
      <ActionButton label="Close" command={closeCommand} icon={X} iconSize={18} className="issue-actions__btn issue-actions__btn--close" onClick={onClose} />
    </div>
  );
}
