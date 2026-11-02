import { useStore } from "../state/store";
import type { Command } from "./registry";
import { isTextInput } from "./registry";

/** What the shown view does with the issue keys; the List registers it while it is on screen. */
export interface IssueKeyHandlers {
  move: (step: 1 | -1) => void;
  /** ⇧J / ⇧K: picks the next row too, where rows can be picked. */
  extend?: (step: 1 | -1) => void;
  /** X: picks the selected row, or drops it from the picks. */
  togglePick?: () => void;
  /** ⌘A: picks every row the view shows. */
  pickAll?: () => void;
  /** Esc, before closing the preview: drops the picks. */
  clearPicks?: () => void;
  hasPicks?: () => boolean;
  /** The selected row's group: ← closes it, → opens it. */
  setGroupOpen: (open: boolean) => void;
  closePeek: () => void;
  copyKey: () => void;
  copyUrl: () => void;
  copyAppLink: () => void;
  openInJira: () => void;
  /** ↩: the selected issue in the full Issue view; absent where the issue is already open. */
  open?: () => void;
  /** ⌘⇧↩: the selected issue's Issue view in a new tab, shown; absent where the issue is already open. */
  openInNewTab?: () => void;
  /** E: the selected issue's epic in the Issue view. */
  openEpic: () => void;
  /** P: bookmarks the selected issue for the default duration, or removes its bookmark. */
  togglePin: () => void;
  /** M: the selected issue is marked read, or unread again when it is read. */
  toggleRead: () => void;
  /** I: the selected issue is assigned to me, or unassigned when it is mine. */
  toggleMine: () => void;
  /** D: the selected issue's status menu. */
  changeStatus: () => void;
  /** A: the selected issue's assignee menu. */
  assignTo: () => void;
  /** ⇧P: the selected issue's priority menu. */
  changePriority: () => void;
  /** F2: edits the selected issue's title where it shows (the Preview or the Issue view). */
  rename: () => void;
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
  { id: "issue.togglePick", title: "Pick the issue or drop it from the picks", scope: "main", defaultBinding: "X", when: () => selected() && handlers?.togglePick !== undefined, run: () => handlers?.togglePick?.() },
  { id: "issue.pickNext", title: "Pick the next issue too", scope: "main", defaultBinding: "Shift+J", when: () => handlers?.extend !== undefined, run: () => handlers?.extend?.(1) },
  { id: "issue.pickNext.arrow", title: "Pick the next issue too (arrow)", scope: "main", defaultBinding: "Shift+ArrowDown", when: () => handlers?.extend !== undefined, run: () => handlers?.extend?.(1) },
  { id: "issue.pickPrevious", title: "Pick the previous issue too", scope: "main", defaultBinding: "Shift+K", when: () => handlers?.extend !== undefined, run: () => handlers?.extend?.(-1) },
  { id: "issue.pickPrevious.arrow", title: "Pick the previous issue too (arrow)", scope: "main", defaultBinding: "Shift+ArrowUp", when: () => handlers?.extend !== undefined, run: () => handlers?.extend?.(-1) },
  // ⌘A stays Edit → Select All in a text field and where no rows can be picked.
  { id: "issue.pickAll", title: "Pick all issues", scope: "main", defaultBinding: "Mod+A", when: () => handlers?.pickAll !== undefined && !isTextInput(document.activeElement) && !document.querySelector("[role=dialog]"), run: () => handlers?.pickAll?.() },
  { id: "group.collapse", title: "Collapse the issue's group", scope: "main", defaultBinding: "ArrowLeft", when: selected, run: () => handlers?.setGroupOpen(false) },
  { id: "group.expand", title: "Expand the issue's group", scope: "main", defaultBinding: "ArrowRight", when: selected, run: () => handlers?.setGroupOpen(true) },
  // Escape in a text field (the title being renamed) leaves the field first. Picks go before the preview; picking
  // closes the preview, so the two never wait at once.
  {
    id: "peek.close",
    title: "Close the preview or drop the picked issues",
    scope: "main",
    defaultBinding: "Escape",
    when: () => ((handlers?.peekOpen() ?? false) || (handlers?.hasPicks?.() ?? false)) && !document.querySelector("[role=dialog]") && !isTextInput(document.activeElement),
    run: () => (handlers?.hasPicks?.() ? handlers.clearPicks?.() : handlers?.closePeek()),
  },
  { id: "issue.copyKey", title: "Copy issue key", scope: "main", defaultBinding: "Mod+C", when: () => selected() && nothingElseToCopy(), run: () => handlers?.copyKey() },
  { id: "issue.copyUrl", title: "Copy issue URL", scope: "main", defaultBinding: "Mod+Shift+C", when: selected, run: () => handlers?.copyUrl() },
  { id: "issue.copyAppLink", title: "Copy Taskboard link", scope: "main", defaultBinding: "Mod+Alt+Shift+C", when: selected, run: () => handlers?.copyAppLink() },
  { id: "issue.open", title: "Open the issue", scope: "main", defaultBinding: "Enter", when: () => selected() && handlers?.open !== undefined, run: () => handlers?.open?.() },
  { id: "issue.openInNewTab", title: "Open the issue in a new tab", scope: "main", defaultBinding: "Mod+Shift+Enter", when: () => selected() && handlers?.openInNewTab !== undefined, run: () => handlers?.openInNewTab?.() },
  { id: "issue.openEpic", title: "Open the issue's epic", scope: "main", defaultBinding: "E", when: selected, run: () => handlers?.openEpic() },
  { id: "issue.togglePin", title: "Bookmark the issue or remove its bookmark", scope: "main", defaultBinding: "P", when: selected, run: () => handlers?.togglePin() },
  { id: "issue.toggleMine", title: "Assign the issue to me or unassign it", scope: "main", defaultBinding: "I", when: selected, run: () => handlers?.toggleMine() },
  { id: "issue.changeStatus", title: "Change the issue's status", scope: "main", defaultBinding: "D", when: selected, run: () => handlers?.changeStatus() },
  { id: "issue.assignTo", title: "Assign the issue to someone", scope: "main", defaultBinding: "A", when: selected, run: () => handlers?.assignTo() },
  { id: "issue.changePriority", title: "Change the issue's priority", scope: "main", defaultBinding: "Shift+P", when: selected, run: () => handlers?.changePriority() },
  { id: "issue.rename", title: "Rename the issue", scope: "main", defaultBinding: "F2", when: selected, run: () => handlers?.rename() },
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
  // ⌘↩ in a text field is the field's (it sends a comment).
  { id: "issue.openInJira", title: "Open issue in Jira", scope: "main", defaultBinding: "Mod+Enter", when: () => selected() && !isTextInput(document.activeElement), run: () => handlers?.openInJira() },
];
