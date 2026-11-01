import { useShortcuts } from "../commands/useShortcuts";

/** The one "Open in Jira" button of the headers: the content header's (a Space) and the Issue view's. `command`, the
 *  command that does the same, puts its key in the tooltip. */
export function OpenInJiraButton({ onClick, command }: { onClick: () => void; command?: string }) {
  const { title } = useShortcuts();
  return (
    <button type="button" className="open-in-jira" title={command ? title("Open in Jira", command) : undefined} onClick={onClick}>
      Open in Jira <span aria-hidden="true">↗</span>
    </button>
  );
}
