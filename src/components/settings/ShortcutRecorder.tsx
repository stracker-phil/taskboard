import { useState, type KeyboardEvent } from "react";

interface Props {
  value: string | null;
  /** The shortcut a key press records; `null` keeps listening (a bare modifier). */
  parse: (e: KeyboardEvent<HTMLButtonElement>) => string | null;
  /** How a recorded shortcut reads, e.g. `⌘⇧K`. */
  label: (shortcut: string) => string;
  onChange: (shortcut: string | null) => void;
  /** Names the field for assistive tech: the command's title. */
  name: string;
}

/** A click records the next key press: ⌫ clears the shortcut, esc cancels. */
export function ShortcutRecorder({ value, parse, label, onChange, name }: Props) {
  const [recording, setRecording] = useState(false);
  const action = recording ? "Press the new shortcut" : "Record shortcut";
  const className = ["shortcut-field", recording && "shortcut-field--recording", value === null && !recording && "shortcut-field--unbound"].filter(Boolean).join(" ");
  return (
    <button
      type="button"
      data-recording={recording || undefined}
      className={className}
      aria-label={`${action} for ${name}`}
      onClick={(e) => {
        e.currentTarget.focus();
        setRecording(true);
      }}
      onBlur={() => setRecording(false)}
      onKeyDown={(e) => {
        if (!recording) return;
        // Tab is never a shortcut: it moves focus on, which ends the recording.
        if (e.key === "Tab") return;
        e.preventDefault();
        e.stopPropagation();
        if (e.key === "Escape") return setRecording(false);
        if (e.key === "Backspace" || e.key === "Delete") {
          onChange(null);
          return setRecording(false);
        }
        const shortcut = parse(e);
        if (!shortcut) return;
        onChange(shortcut);
        setRecording(false);
      }}
    >
      {recording ? "Press keys…" : value ? label(value) : "unbound"}
    </button>
  );
}
