import { Save } from "lucide-react";

/** Shown while the view has unsaved filter changes (ADR 0038): writes them to the view; leaving drops them. */
export function SaveViewButton({ onSave }: { onSave: () => void }) {
  return (
    <button type="button" className="toolbar-btn save-view-btn" title="Keep the Me chip and the filter in this view" onClick={onSave}>
      <Save size={14} strokeWidth={2} aria-hidden="true" />
      Save View
    </button>
  );
}
