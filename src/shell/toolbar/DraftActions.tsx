import { Save, Undo2 } from "lucide-react";
import { useShortcuts } from "../../commands/useShortcuts";
import { usePresence } from "../../lib/motion";

/** Revert View and Save View, shown while the view has unsaved filter changes (ADR 0038). They appear in the bar's
 *  free middle, so nothing else moves when they come or go; leaving the view drops the changes. */
export function DraftActions({ dirty, onSave, onRevert }: { dirty: boolean; onSave: () => void; onRevert: () => void }) {
  const { mounted, state } = usePresence(dirty);
  const { title } = useShortcuts();
  if (!mounted) return null;
  return (
    <span className={`draft-actions draft-actions--${state}`} inert={state === "leaving"}>
      <button type="button" className="toolbar-btn toolbar-btn--icon" aria-label="Revert View" title={title("Revert View: back to what the view saved", "view.revert")} onClick={onRevert}>
        <Undo2 size={14} strokeWidth={2} aria-hidden="true" />
      </button>
      <button type="button" className="toolbar-btn save-view-btn" title={title("Save View: keep the filter and the chips in this view", "view.save")} onClick={onSave}>
        <Save size={14} strokeWidth={2} aria-hidden="true" />
        Save View
      </button>
    </span>
  );
}
