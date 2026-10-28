import { Search } from "lucide-react";
import { type Ref, useEffect, useId, useState } from "react";

const DEBOUNCE_MS = 80;

/** "Filter this view": narrows by key or summary as you type. */
export function TextFilter({ value, onChange, inputRef }: { value: string; onChange: (text: string) => void; inputRef?: Ref<HTMLInputElement> }) {
  const id = useId();
  const [draft, setDraft] = useState(value);
  useEffect(() => {
    if (draft === value) return;
    const timer = setTimeout(() => onChange(draft), DEBOUNCE_MS);
    return () => clearTimeout(timer);
  }, [draft, value, onChange]);
  return (
    <label className="text-filter" htmlFor={id}>
      <Search size={14} strokeWidth={2} aria-hidden="true" />
      <span className="visually-hidden">Filter this view</span>
      <input ref={inputRef} id={id} type="text" placeholder="Filter this view" value={draft} onChange={(e) => setDraft(e.target.value)} />
    </label>
  );
}
