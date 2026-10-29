import type { LucideIcon } from "lucide-react";
import { useRef, type ReactNode } from "react";
import { LookPopover } from "./LookPopover";
import type { Look } from "./Tile";

interface Props {
  /** The button's accessible name. */
  label: string;
  className: string;
  look: Look;
  fallback: string | LucideIcon;
  colorOptional?: boolean;
  /** Whether the look popover is open. */
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Set while its item isn't selected: a click selects the item instead of editing the look. */
  onSelect?: () => void;
  onChange: (look: Look) => void;
  /** The tile it shows. */
  children: ReactNode;
}

/** A tile that edits its own look: a click opens the look popover under it. */
export function LookButton({ label, className, look, fallback, colorOptional, open, onOpenChange, onSelect, onChange, children }: Props) {
  const ref = useRef<HTMLButtonElement>(null);
  return (
    <>
      <button
        ref={ref}
        type="button"
        className={className}
        aria-label={label}
        aria-expanded={onSelect ? undefined : open}
        onClick={onSelect ?? (() => onOpenChange(!open))}
      >
        {children}
      </button>
      {open && (
        <LookPopover
          title="Icon and color"
          look={look}
          fallback={fallback}
          colorOptional={colorOptional}
          anchor={ref}
          onChange={onChange}
          onClose={() => onOpenChange(false)}
        />
      )}
    </>
  );
}
