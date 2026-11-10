import type { Command } from "../commands/registry";

/** What the open inbox does with its keys. */
export interface InboxHandlers {
  move: (step: 1 | -1) => void;
  toggleRead: () => void;
  open: (inNewTab: boolean) => void;
  markAllRead: () => void;
  togglePreview: () => void;
  /** Closes the preview, or the inbox when none shows. */
  close: () => void;
}

let handlers: InboxHandlers | null = null;

/** The open inbox takes its keys; returns the release. */
export function onInboxKeys(next: InboxHandlers): () => void {
  handlers = next;
  return () => {
    if (handlers === next) handlers = null;
  };
}

const key = (id: string, title: string, defaultBinding: string, run: (h: InboxHandlers) => void): Command => ({
  id,
  title,
  scope: "inbox",
  defaultBinding,
  when: () => handlers !== null,
  run: () => {
    if (handlers) run(handlers);
  },
});

/** The inbox's own keys: they win over the list's while it is open (`inbox` scope). */
export const INBOX_KEY_COMMANDS: Command[] = [
  key("inbox.next", "Next notification", "J", (h) => h.move(1)),
  key("inbox.down", "Next notification (arrow)", "ArrowDown", (h) => h.move(1)),
  key("inbox.previous", "Previous notification", "K", (h) => h.move(-1)),
  key("inbox.up", "Previous notification (arrow)", "ArrowUp", (h) => h.move(-1)),
  key("inbox.toggleRead", "Mark as read or unread", "M", (h) => h.toggleRead()),
  key("inbox.openIssue", "Open the issue", "Enter", (h) => h.open(false)),
  key("inbox.openInNewTab", "Open the issue in a new tab", "Mod+Enter", (h) => h.open(true)),
  key("inbox.markAllRead", "Mark all as read", "Mod+Shift+A", (h) => h.markAllRead()),
  key("inbox.preview", "Preview the issue", "Space", (h) => h.togglePreview()),
  key("inbox.close", "Close notifications", "Escape", (h) => h.close()),
];
