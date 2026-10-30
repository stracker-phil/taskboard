import { Search } from "lucide-react";
import { type KeyboardEvent, type Ref, useEffect, useId, useState } from "react";

const DEBOUNCE_MS = 80;

interface Props {
  value: string;
  onChange: (text: string) => void;
  inputRef?: Ref<HTMLInputElement>;
  /** The placeholder and the field's name. */
  label?: string;
  /** Enter (backwards with ⇧), with the text as typed. */
  onEnter?: (text: string, backwards: boolean) => void;
  /** Shown at the field's end, in a slot that keeps its width. */
  status?: string;
}

/** The text field of the toolbar ("Filter this view") and the Issue view's find. Escape clears it, then leaves it. */
export function TextFilter({ value, onChange, inputRef, label = "Filter this view", onEnter, status }: Props) {
  const id = useId();
  const [draft, setDraft] = useState(value);
  useEffect(() => {
    if (draft === value) return;
    const timer = setTimeout(() => onChange(draft), DEBOUNCE_MS);
    return () => clearTimeout(timer);
  }, [draft, value, onChange]);

  const onKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" && onEnter) {
      e.preventDefault();
      if (draft !== value) onChange(draft);
      onEnter(draft, e.shiftKey);
    } else if (e.key === "Escape") {
      if (draft) {
        setDraft("");
        onChange("");
      } else e.currentTarget.blur();
    }
  };

  return (
    <label className="text-filter" htmlFor={id}>
      <Search size={14} strokeWidth={2} aria-hidden="true" />
      <input ref={inputRef} id={id} type="text" aria-label={label} placeholder={label} value={draft} onChange={(e) => setDraft(e.target.value)} onKeyDown={onKeyDown} />
      {status !== undefined && (
        <span className="text-filter__status" aria-live="polite">
          {status}
        </span>
      )}
    </label>
  );
}
