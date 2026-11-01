import { ChevronDown } from "lucide-react";
import type { RefObject } from "react";
import { useConnection } from "../../queries/settings";

/** Initials of the connected account ("Alex Morgan" → "AM"). */
function initials(name: string): string {
  const words = name.trim().split(/\s+/).filter(Boolean);
  const picked = words.length > 1 ? [words[0], words[words.length - 1]] : words;
  return picked.map((w) => w.charAt(0).toUpperCase()).join("");
}

interface Props {
  on: boolean;
  /** Me mode holds in every view, not only this one. */
  everywhere: boolean;
  onChange: (on: boolean) => void;
  /** The options button, which the Me popover hangs under. */
  optionsRef: RefObject<HTMLButtonElement | null>;
  optionsOpen: boolean;
  onOptions: () => void;
}

function chipTitle(on: boolean, everywhere: boolean): string {
  if (!on) return "Show only my issues";
  return everywhere ? "My issues in every view; click to show all" : "Only my issues; click to show all";
}

/** On and off differ only in color, never in size, so either half stays put under the cursor (docs/ux.md →
 *  Stable controls). */
export function MeChip({ on, everywhere, onChange, optionsRef, optionsOpen, onOptions }: Props) {
  const { data } = useConnection();
  const name = data?.connection.displayName ?? "";
  return (
    <span className={on ? "me-split me-split--on" : "me-split"}>
      <button type="button" className="me-split__toggle" aria-pressed={on} title={chipTitle(on, everywhere)} onClick={() => onChange(!on)}>
        <span className={`avatar avatar--1 me-chip__avatar${on && everywhere ? " me-chip__avatar--everywhere" : ""}`} aria-hidden="true">
          {initials(name)}
        </span>
        Me
      </button>
      <button ref={optionsRef} type="button" className="me-split__options" aria-label="Me mode options" aria-expanded={optionsOpen} onClick={onOptions}>
        <ChevronDown size={12} strokeWidth={2.25} aria-hidden="true" />
      </button>
    </span>
  );
}
