import { Bell } from "lucide-react";
import { useRef } from "react";
import { usePageAction } from "../commands/pageActions";
import { useShortcuts } from "../commands/useShortcuts";
import { InboxDropdown } from "./InboxDropdown";
import { useInboxState } from "./inboxState";
import { useInboxCount, useInboxToggle } from "./queries";

/** The bell in the content panel's top-right corner, on every page: the unread count, and the inbox under it. */
export function InboxButton() {
  const open = useInboxState((s) => s.open);
  const count = useInboxCount();
  const { show, toggle, close } = useInboxToggle();
  const { title } = useShortcuts();
  const ref = useRef<HTMLButtonElement>(null);
  usePageAction("inbox.toggle", toggle);
  usePageAction("inbox.open", () => void (useInboxState.getState().open || show()));
  const label = count === 0 ? "Notifications" : `Notifications, ${count} unread`;
  return (
    <div className="inbox-slot">
      <button
        ref={ref}
        type="button"
        className={open ? "inbox-bell inbox-bell--open" : "inbox-bell"}
        aria-label={label}
        aria-expanded={open}
        aria-haspopup="dialog"
        title={title("Notifications", "inbox.toggle")}
        onClick={toggle}
      >
        <Bell size={16} strokeWidth={1.8} aria-hidden="true" />
        {count > 0 && <span className="inbox-bell__badge">{count > 99 ? "99+" : count}</span>}
      </button>
      {open && <InboxDropdown bell={ref} onClose={close} />}
    </div>
  );
}
