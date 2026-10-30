import { useConnection } from "../../queries/settings";

/** Initials of the connected account ("Alex Morgan" → "AM"). */
function initials(name: string): string {
  const words = name.trim().split(/\s+/).filter(Boolean);
  const picked = words.length > 1 ? [words[0], words[words.length - 1]] : words;
  return picked.map((w) => w.charAt(0).toUpperCase()).join("");
}

/** Narrows to issues assigned to me. On and off look different only in color, never in size, so it can be
 *  clicked again at the same spot (docs/ux.md → Stable controls). */
export function MeChip({ on, onChange }: { on: boolean; onChange: (on: boolean) => void }) {
  const { data } = useConnection();
  const name = data?.connection.displayName ?? "";
  return (
    <button type="button" className="me-chip" aria-pressed={on} title={on ? "Only my issues; click to show all" : "Show only my issues"} onClick={() => onChange(!on)}>
      <span className="avatar avatar--1 me-chip__avatar" aria-hidden="true">
        {initials(name)}
      </span>
      Me
    </button>
  );
}
