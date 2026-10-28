import { X } from "lucide-react";
import { useConnection } from "../../queries/settings";

/** Initials of the connected account ("Alex Morgan" → "AM"). */
function initials(name: string): string {
  const words = name.trim().split(/\s+/).filter(Boolean);
  const picked = words.length > 1 ? [words[0], words[words.length - 1]] : words;
  return picked.map((w) => w.charAt(0).toUpperCase()).join("");
}

/** Narrows to issues assigned to me: "Me" when off, the avatar and "Me ×" when on. */
export function MeChip({ on, onChange }: { on: boolean; onChange: (on: boolean) => void }) {
  const { data } = useConnection();
  const name = data?.connection.displayName ?? "";
  if (!on) {
    return (
      <button type="button" className="toolbar-btn" aria-pressed={false} onClick={() => onChange(true)}>
        Me
      </button>
    );
  }
  return (
    <button type="button" className="me-chip" aria-pressed={true} aria-label="Only my issues, click to show all" onClick={() => onChange(false)}>
      <span className="avatar avatar--1 me-chip__avatar" aria-hidden="true">
        {initials(name)}
      </span>
      Me
      <X size={12} strokeWidth={2.2} aria-hidden="true" />
    </button>
  );
}
