import { useStore } from "../../state/store";

/** A click on the selected Space or list: closes the open Issue view (as esc does), or else renames it. */
export function useSelectedClick(onRename: () => void) {
  const issueOpen = useStore((s) => s.issueView !== null);
  const closeIssue = useStore((s) => s.closeIssue);
  return {
    title: issueOpen ? "Back to the view" : "Click to rename",
    click: issueOpen ? closeIssue : onRename,
  };
}
