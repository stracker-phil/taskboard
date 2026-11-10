import { Search } from "lucide-react";

/** The search box on top of a long settings list; `label` is both its accessible name and its placeholder. */
export function SearchField({ label, value, onChange }: { label: string; value: string; onChange: (value: string) => void }) {
  return (
    <div className="settings-row settings-list-search">
      <label className="columns-popover__search settings-list-search__field">
        <Search size={13} strokeWidth={2} aria-hidden="true" />
        <input aria-label={label} placeholder={label} value={value} onChange={(e) => onChange(e.target.value)} />
      </label>
    </div>
  );
}
