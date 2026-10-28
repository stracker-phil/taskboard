import type { Command } from "./registry";
import { isTextInput } from "./registry";

/** What the shown view does with the issue keys; the List registers it while it is on screen. */
export interface IssueKeyHandlers {
  move: (step: 1 | -1) => void;
  /** The selected row's group: ← closes it, → opens it. */
  setGroupOpen: (open: boolean) => void;
  closePeek: () => void;
  copyKey: () => void;
  copyUrl: () => void;
  openInJira: () => void;
  hasSelection: () => boolean;
  peekOpen: () => boolean;
}

let handlers: IssueKeyHandlers | null = null;

/** Takes the issue keys while mounted; returns the release. */
export function onIssueKeys(next: IssueKeyHandlers): () => void {
  handlers = next;
  return () => {
    if (handlers === next) handlers = null;
  };
}

const selected = () => handlers?.hasSelection() ?? false;
/** ⌘C copies the key only when nothing else would be copied: no text field focused, no text selected. */
const nothingElseToCopy = () => !isTextInput(document.activeElement) && (window.getSelection()?.isCollapsed ?? true);

export const ISSUE_KEY_COMMANDS: Command[] = [
  { id: "issue.next", title: "Next issue", scope: "main", defaultBinding: "J", when: () => handlers !== null, run: () => handlers?.move(1) },
  { id: "issue.next.arrow", title: "Next issue (arrow)", scope: "main", defaultBinding: "ArrowDown", when: () => handlers !== null, run: () => handlers?.move(1) },
  { id: "issue.previous", title: "Previous issue", scope: "main", defaultBinding: "K", when: () => handlers !== null, run: () => handlers?.move(-1) },
  { id: "issue.previous.arrow", title: "Previous issue (arrow)", scope: "main", defaultBinding: "ArrowUp", when: () => handlers !== null, run: () => handlers?.move(-1) },
  { id: "group.collapse", title: "Collapse the issue's group", scope: "main", defaultBinding: "ArrowLeft", when: selected, run: () => handlers?.setGroupOpen(false) },
  { id: "group.expand", title: "Expand the issue's group", scope: "main", defaultBinding: "ArrowRight", when: selected, run: () => handlers?.setGroupOpen(true) },
  { id: "peek.close", title: "Close the peek", scope: "main", defaultBinding: "Escape", when: () => (handlers?.peekOpen() ?? false) && !document.querySelector("[role=dialog]"), run: () => handlers?.closePeek() },
  { id: "issue.copyKey", title: "Copy issue key", scope: "main", defaultBinding: "Mod+C", when: () => selected() && nothingElseToCopy(), run: () => handlers?.copyKey() },
  { id: "issue.copyUrl", title: "Copy issue URL", scope: "main", defaultBinding: "Mod+Shift+C", when: selected, run: () => handlers?.copyUrl() },
  { id: "issue.openInJira", title: "Open issue in Jira", scope: "main", defaultBinding: "Mod+Enter", when: selected, run: () => handlers?.openInJira() },
];
