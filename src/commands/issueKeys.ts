import { useStore } from "../state/store";
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
  /** ↩: the selected issue in the full Issue view; absent where the issue is already open. */
  open?: () => void;
  /** E: the selected issue's epic in the Issue view. */
  openEpic: () => void;
  /** P: bookmarks the selected issue for the default duration, or removes its bookmark. */
  togglePin: () => void;
  /** M: the selected issue is marked read, or unread again when it is read. */
  toggleRead: () => void;
  /** Space: opens or closes the selected issue's preview; absent where there is none, so Space scrolls. */
  togglePeek?: () => void;
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
  { id: "peek.close", title: "Close the preview", scope: "main", defaultBinding: "Escape", when: () => (handlers?.peekOpen() ?? false) && !document.querySelector("[role=dialog]"), run: () => handlers?.closePeek() },
  { id: "issue.copyKey", title: "Copy issue key", scope: "main", defaultBinding: "Mod+C", when: () => selected() && nothingElseToCopy(), run: () => handlers?.copyKey() },
  { id: "issue.copyUrl", title: "Copy issue URL", scope: "main", defaultBinding: "Mod+Shift+C", when: selected, run: () => handlers?.copyUrl() },
  { id: "issue.open", title: "Open the issue", scope: "main", defaultBinding: "Enter", when: () => selected() && handlers?.open !== undefined, run: () => handlers?.open?.() },
  { id: "issue.openEpic", title: "Open the issue's epic", scope: "main", defaultBinding: "E", when: selected, run: () => handlers?.openEpic() },
  { id: "issue.togglePin", title: "Bookmark the issue or remove its bookmark", scope: "main", defaultBinding: "P", when: selected, run: () => handlers?.togglePin() },
  { id: "issue.markSeen", title: "Mark the issue as read or unread", scope: "main", defaultBinding: "M", when: selected, run: () => handlers?.toggleRead() },
  { id: "issue.togglePeek", title: "Open or close the preview", scope: "main", defaultBinding: "Space", when: () => selected() && handlers?.togglePeek !== undefined, run: () => handlers?.togglePeek?.() },
  {
    id: "issue.back",
    title: "Back from the Issue view",
    scope: "main",
    defaultBinding: "Escape",
    // Escape in a text field (the find) clears or leaves it first.
    when: () => useStore.getState().issueView !== null && !document.querySelector("[role=dialog]") && !isTextInput(document.activeElement),
    run: () => useStore.getState().issueBack(),
  },
  { id: "issue.openInJira", title: "Open issue in Jira", scope: "main", defaultBinding: "Mod+Enter", when: selected, run: () => handlers?.openInJira() },
];
