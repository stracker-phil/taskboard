import { Bookmark, CheckCheck, CircleDot, Flag, UserRound, X } from "lucide-react";
import { useState, type ReactNode } from "react";
import { api } from "../api";
import { useShortcuts } from "../commands/useShortcuts";
import { usePresence } from "../lib/motion";
import { usePinDefault } from "../pins/PinPopover";
import { attempt } from "../state/requests";
import { useStore } from "../state/store";
import { usePinnedIssues } from "../views/list/usePinnedIssues";
import { openBulkAssignMenu, openBulkPriorityMenu, openBulkStatusMenu } from "./bulkMenus";
import { explainWritesOff } from "./runWrite";
import { useJiraChange } from "./useJiraChanges";

type Menu = "status" | "assign" | "priority";

/** D, A or ⇧P while issues are picked: the bar's own button opens its menu, so the key does what the click does. */
export function openBulkMenu(menu: Menu) {
  const button = document.querySelector<HTMLButtonElement>(`[data-bulk-menu="${menu}"]`);
  // Another menu's choices are still loading.
  if (!button || button.closest(".bulk-bar")?.querySelector('[aria-busy="true"]')) return;
  if (button.disabled) explainWritesOff();
  else button.click();
}

/** The bar over the bottom of a view while issues are picked (docs/ux.md → Picking issues): what to do with all
 *  of them. Every button keeps its slot; one whose change is switched off is greyed out. */
export function BulkBar({ picked, onClear }: { picked: string[]; onClear: () => void }) {
  const { mounted, state } = usePresence(picked.length > 0);
  // While it fades out it keeps showing the picks it had.
  const [shown, setShown] = useState(picked);
  if (picked.length > 0 && picked !== shown) setShown(picked);
  if (!mounted) return null;
  return <Bar ids={shown} leaving={state === "leaving"} onClear={onClear} />;
}

function Bar({ ids, leaving, onClear }: { ids: string[]; leaving: boolean; onClear: () => void }) {
  const { title } = useShortcuts();
  const [busy, setBusy] = useState<Menu | null>(null);
  const canStatus = useJiraChange("changeStatus");
  const canAssign = useJiraChange("assign");
  const canPriority = useJiraChange("changePriority");
  const pinned = usePinnedIssues();
  const pinDefault = usePinDefault();
  const toast = useStore((s) => s.toast);
  const marking = (on: boolean, menu: Menu) => setBusy(on ? menu : null);

  // Both commands answer nothing (null), so success is marked before `attempt` turns a failure into null.
  const markRead = () =>
    void attempt(api.issuesMarkSeen(ids).then(() => true), "Could not mark them read").then((done) => {
      if (done) toast("success", ids.length === 1 ? "Marked 1 issue read" : `Marked ${ids.length} issues read`);
    });
  const bookmark = async () => {
    const missing = ids.filter((id) => !pinned.has(id));
    if (missing.length === 0) return toast("info", "They are all bookmarked already");
    for (const id of missing) {
      if (!(await attempt(api.pinCreate("issue", id, pinDefault, null).then(() => true), "Could not bookmark it"))) return;
    }
    toast("success", missing.length === 1 ? "Bookmarked 1 issue" : `Bookmarked ${missing.length} issues`);
  };

  return (
    <div className={leaving ? "bulk-bar bulk-bar--leaving" : "bulk-bar"} role="toolbar" aria-label="Picked issues" inert={leaving}>
      <span className="bulk-bar__count">{ids.length === 1 ? "1 picked" : `${ids.length} picked`}</span>
      <span className="bulk-bar__divider" aria-hidden="true" />
      <BarButton menu="status" icon={<CircleDot size={15} />} label="Status" title={title("Change their status", "issue.changeStatus")} enabled={canStatus} busy={busy} run={(at) => openBulkStatusMenu(ids, at, (on) => marking(on, "status"))} />
      <BarButton menu="assign" icon={<UserRound size={15} />} label="Assign" title={title("Assign them", "issue.assignTo")} enabled={canAssign} busy={busy} run={(at) => openBulkAssignMenu(ids, at, (on) => marking(on, "assign"))} />
      <BarButton menu="priority" icon={<Flag size={15} />} label="Priority" title={title("Set their priority", "issue.changePriority")} enabled={canPriority} busy={busy} run={(at) => openBulkPriorityMenu(ids, at, (on) => marking(on, "priority"))} />
      <span className="bulk-bar__divider" aria-hidden="true" />
      <button type="button" className="toolbar-btn" onClick={markRead}>
        <CheckCheck size={15} aria-hidden="true" />
        Mark Read
      </button>
      <button type="button" className="toolbar-btn" onClick={() => void bookmark()}>
        <Bookmark size={15} aria-hidden="true" />
        Bookmark
      </button>
      <span className="bulk-bar__divider" aria-hidden="true" />
      <button type="button" className="toolbar-btn toolbar-btn--icon" aria-label="Drop the picks" title={title("Drop the picks", "peek.close")} onClick={onClear}>
        <X size={15} aria-hidden="true" />
      </button>
    </div>
  );
}

interface BarButtonProps {
  menu: Menu;
  icon: ReactNode;
  label: string;
  title: string;
  enabled: boolean;
  busy: Menu | null;
  run: (at: HTMLElement) => void;
}

/** A button that opens a bulk menu; pressed while its choices load. Switched off, it stays in its slot, greyed. */
function BarButton({ menu, icon, label, title, enabled, busy, run }: BarButtonProps) {
  return (
    <button
      type="button"
      className={busy === menu ? "toolbar-btn toolbar-btn--active" : "toolbar-btn"}
      data-bulk-menu={menu}
      aria-haspopup="menu"
      aria-busy={busy === menu}
      title={enabled ? title : `${label}: switched off in Settings → Connection`}
      disabled={!enabled || busy !== null}
      onClick={(e) => run(e.currentTarget)}
    >
      <span aria-hidden="true">{icon}</span>
      {label}
    </button>
  );
}
