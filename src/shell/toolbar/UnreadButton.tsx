import { BellDot } from "lucide-react";
import { forwardRef } from "react";
import { useShortcuts } from "../../commands/useShortcuts";

/** Unread only, for every view and the sidebar's counts at once; on and off differ only in color. */
export const UnreadButton = forwardRef<HTMLButtonElement, { on: boolean; onChange: (on: boolean) => void }>(function UnreadButton({ on, onChange }, ref) {
  const { title } = useShortcuts();
  return (
    <button
      ref={ref}
      type="button"
      className={on ? "toolbar-btn toolbar-btn--icon toolbar-btn--active" : "toolbar-btn toolbar-btn--icon"}
      aria-label="Unread only"
      aria-pressed={on}
      title={title("Only unread issues, in every view", "toolbar.unread")}
      onClick={() => onChange(!on)}
    >
      <BellDot size={14} strokeWidth={2} aria-hidden="true" />
    </button>
  );
});
