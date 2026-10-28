import { useRef, type KeyboardEvent } from "react";

interface Props {
  value: string;
  /** What the field is for, its accessible name. */
  label: string;
  /** Called with the trimmed name; only when it is not empty and changed. */
  onSave: (name: string) => void;
  /** Called after a save as well, to leave the field. */
  onDone: () => void;
  className?: string;
}

/** A name edited in place: ↩ and leaving the field save, esc cancels. */
export function InlineRename({ value, label, onSave, onDone, className }: Props) {
  const finished = useRef(false);

  const finish = (name: string | null) => {
    if (finished.current) return;
    finished.current = true;
    const trimmed = name?.trim();
    if (trimmed && trimmed !== value) onSave(trimmed);
    onDone();
  };

  const onKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") finish(e.currentTarget.value);
    else if (e.key === "Escape") finish(null);
    else return;
    e.preventDefault();
    e.stopPropagation();
  };

  return (
    <input
      className={className ? `inline-rename ${className}` : "inline-rename"}
      aria-label={label}
      defaultValue={value}
      autoFocus
      onFocus={(e) => e.currentTarget.select()}
      onKeyDown={onKeyDown}
      onBlur={(e) => finish(e.currentTarget.value)}
      onClick={(e) => e.stopPropagation()}
      onPointerDown={(e) => e.stopPropagation()}
    />
  );
}
