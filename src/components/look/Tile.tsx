import type { LucideIcon } from "lucide-react";
import { lineIcon } from "./icons";

/** A tile's look as Rust stores it (`look.rs`): an icon (a letter, digit or emoji, or a line icon's name) and a
 *  tile color number. */
export interface Look {
  icon: string | null;
  color: string | null;
}

interface Props extends Look {
  /** Shown without an icon: a letter (a Space's initial) or a line icon (the folder, a layout). */
  fallback: string | LucideIcon;
  size: number;
}

const radius = (size: number) => (size >= 28 ? 8 : size >= 20 ? 6 : 5);

/** On its color (`--space-tile-N`) a white letter, emoji or line icon; without a color the icon alone, in the
 *  text color. */
export function Tile({ icon, color, fallback, size }: Props) {
  const Line = lineIcon(icon) ?? (!icon && typeof fallback !== "string" ? fallback : null);
  const glyph = icon ?? (typeof fallback === "string" ? fallback : "");
  const content = Line ? (
    <Line size={Math.round(color ? size * 0.62 : size)} strokeWidth={2} />
  ) : (
    glyph.toUpperCase()
  );
  if (!color) {
    return (
      <span className="tile tile--plain" style={{ width: size, height: size, fontSize: Math.round(size * 0.85) }} aria-hidden="true">
        {content}
      </span>
    );
  }
  return (
    <span
      className="tile"
      style={{ width: size, height: size, borderRadius: radius(size), fontSize: Math.round(size * 0.5), background: `var(--space-tile-${color})` }}
      aria-hidden="true"
    >
      {content}
    </span>
  );
}
