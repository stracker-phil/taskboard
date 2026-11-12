import { ChevronsUpDown } from "lucide-react";
import { useState } from "react";
import { useStore } from "../../state/store";

export interface PopUpOption<T extends string> {
  value: T;
  label: string;
}

interface Props<T extends string> {
  value: T;
  options: readonly PopUpOption<T>[];
  onChange: (value: T) => void;
  /** For a `<label htmlFor>` naming it; the label also names its menu. */
  id?: string;
  disabled?: boolean;
  className?: string;
  /** A search field's placeholder, for a long list of choices. */
  search?: string;
}

/** A macOS pop-up button on the app's menu; its width comes from its container, so a choice never resizes it. */
export function PopUpButton<T extends string>({ value, options, onChange, id, disabled, className, search }: Props<T>) {
  const [button, setButton] = useState<HTMLButtonElement | null>(null);
  const open = useStore((s) => button !== null && s.menu?.at === button);
  const current = options.find((o) => o.value === value);

  const show = () => {
    if (!button) return;
    const entries = options.map((o) => ({
      text: o.label,
      checked: o.value === value,
      run: () => {
        if (o.value !== value) onChange(o.value);
      },
    }));
    useStore.getState().openMenu(entries, button, button.labels?.[0]?.textContent ?? undefined, { search });
  };

  return (
    <button
      ref={setButton}
      type="button"
      id={id}
      className={className ? `popup-button ${className}` : "popup-button"}
      aria-haspopup="menu"
      aria-expanded={open}
      disabled={disabled}
      onClick={show}
      onKeyDown={(e) => {
        if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
        e.preventDefault();
        if (!open) show();
      }}
    >
      <span className="popup-button__value">{current?.label ?? ""}</span>
      <ChevronsUpDown className="popup-button__chevron" size={12} strokeWidth={2.2} aria-hidden="true" />
    </button>
  );
}
