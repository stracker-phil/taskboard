import { Check, ChevronDown, Search } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { Choice } from "../../api";

interface Props {
  id: string;
  choices: Choice[];
  selected: string[];
  /** Runs on every tick, so the view refreshes as values are picked. */
  onChange: (values: string[]) => void;
}

/** A condition's values: the field shows the picked names; clicking it opens a searchable list with a
 *  checkbox per value. As many values as wanted; each tick applies at once. */
export function ValuePicker({ id, choices, selected, onChange }: Props) {
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");
  // The values picked when the list opened lead it; ticking one only changes its box, never the order.
  const [leading, setLeading] = useState<string[]>([]);
  const rootRef = useRef<HTMLDivElement>(null);
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!open) return;
    searchRef.current?.focus();
    const close = (e: MouseEvent) => {
      if (!rootRef.current?.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", close);
    return () => document.removeEventListener("mousedown", close);
  }, [open]);

  const names = selected.map((value) => choices.find((c) => c.id === value)?.name ?? value);
  const needle = search.trim().toLowerCase();
  const shown = needle ? choices.filter((c) => c.name.toLowerCase().includes(needle)) : choices;
  const ordered = [...shown.filter((c) => leading.includes(c.id)), ...shown.filter((c) => !leading.includes(c.id))];
  const toggle = (value: string) =>
    onChange(selected.includes(value) ? selected.filter((v) => v !== value) : choices.map((c) => c.id).filter((c) => c === value || selected.includes(c)));

  return (
    <div ref={rootRef} className="value-picker">
      <button
        id={id}
        type="button"
        className="condition-row__value value-picker__field"
        aria-haspopup="listbox"
        aria-expanded={open}
        title={names.join(", ")}
        onClick={() => {
          if (!open) setLeading(selected);
          setOpen(!open);
        }}
      >
        <span className="value-picker__names">{names.length > 0 ? names.join(", ") : "Choose…"}</span>
        {names.length > 1 && <span className="value-picker__count">{names.length}</span>}
        <ChevronDown size={13} strokeWidth={2} aria-hidden="true" />
      </button>
      {open && (
        <div className="value-picker__panel">
          <label className="value-picker__search">
            <Search size={13} strokeWidth={2} aria-hidden="true" />
            <input ref={searchRef} aria-label="Search values" placeholder="Search" value={search} onChange={(e) => setSearch(e.target.value)} />
          </label>
          <div className="value-picker__list" role="listbox" aria-multiselectable="true" aria-label="Values">
            {ordered.length === 0 && <p className="value-picker__empty">No value matches.</p>}
            {ordered.map((choice) => {
              const on = selected.includes(choice.id);
              return (
                <button key={choice.id} type="button" role="option" aria-selected={on} className="value-picker__option" onClick={() => toggle(choice.id)}>
                  <span className={on ? "value-picker__box value-picker__box--on" : "value-picker__box"} aria-hidden="true">
                    {on && <Check size={11} strokeWidth={3} />}
                  </span>
                  <span className="value-picker__name">{choice.name}</span>
                </button>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}
