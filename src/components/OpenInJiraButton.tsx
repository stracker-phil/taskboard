/** The one "Open in Jira" button of the headers: the content header's (a Space) and the Issue view's. */
export function OpenInJiraButton({ onClick }: { onClick: () => void }) {
  return (
    <button type="button" className="open-in-jira" onClick={onClick}>
      Open in Jira <span aria-hidden="true">↗</span>
    </button>
  );
}
