import type { LucideIcon } from "lucide-react";
import { useState } from "react";
import { BufferedInput } from "../settings/fields";
import { LIST_ICONS, lineIcon } from "./icons";
import { type Look, Tile } from "./Tile";

/** The tile palette (`--space-tile-1..12`), by the color number Rust stores. */
export const TILE_COLORS = [
  { id: "1", label: "Blue" },
  { id: "2", label: "Slate" },
  { id: "3", label: "Violet" },
  { id: "4", label: "Teal" },
  { id: "5", label: "Amber" },
  { id: "6", label: "Red" },
  { id: "7", label: "Pink" },
  { id: "8", label: "Green" },
  { id: "9", label: "Indigo" },
  { id: "10", label: "Cyan" },
  { id: "11", label: "Orange" },
  { id: "12", label: "Stone" },
] as const;

type Kind = "icons" | "glyph";

interface Props {
  look: Look;
  fallback: string | LucideIcon;
  /** Lists and views may go without a color (a plain icon); a Space always has one. */
  colorOptional?: boolean;
  onChange: (look: Look) => void;
}

/** Pick a line icon or type a letter or emoji, and a tile color; every choice saves at once. */
export function LookEditor({ look, fallback, colorOptional = false, onChange }: Props) {
  const [kind, setKind] = useState<Kind>(look.icon && !lineIcon(look.icon) ? "glyph" : "icons");
  return (
    <div className="look-editor">
      <div className="look-editor__head">
        <Tile icon={look.icon} color={look.color ?? (colorOptional ? null : "1")} fallback={fallback} size={32} />
        <div className="segmented" role="group" aria-label="Icon kind">
          <button type="button" aria-pressed={kind === "icons"} onClick={() => setKind("icons")}>
            Icon
          </button>
          <button type="button" aria-pressed={kind === "glyph"} onClick={() => setKind("glyph")}>
            Letter or emoji
          </button>
        </div>
        <button type="button" className="link-btn look-editor__reset" disabled={look.icon === null} onClick={() => onChange({ ...look, icon: null })}>
          Reset
        </button>
      </div>
      {kind === "icons" ? (
        <div className="look-editor__grid" role="radiogroup" aria-label="Icon">
          {Object.entries(LIST_ICONS).map(([name, Icon]) => (
            <button
              key={name}
              type="button"
              role="radio"
              aria-checked={look.icon === name}
              aria-label={name.replace(/-/g, " ")}
              className="look-editor__option"
              onClick={() => onChange({ ...look, icon: name })}
            >
              <Icon size={16} strokeWidth={1.9} aria-hidden="true" />
            </button>
          ))}
        </div>
      ) : (
        <label className="look-editor__glyph">
          <span>One letter, digit or emoji (⌃⌘Space opens the emoji picker)</span>
          <BufferedInput
            className="input"
            aria-label="Letter or emoji"
            value={look.icon && !lineIcon(look.icon) ? look.icon : ""}
            onCommit={(text) => onChange({ ...look, icon: firstGlyph(text) })}
          />
        </label>
      )}
      <div className="look-editor__colors" role="radiogroup" aria-label="Color">
        {colorOptional && (
          <button type="button" role="radio" aria-checked={look.color === null} aria-label="No color" className="look-swatch look-swatch--none" onClick={() => onChange({ ...look, color: null })} />
        )}
        {TILE_COLORS.map((c) => (
          <button
            key={c.id}
            type="button"
            role="radio"
            aria-checked={look.color === c.id}
            aria-label={c.label}
            className="look-swatch"
            style={{ background: `var(--space-tile-${c.id})` }}
            onClick={() => onChange({ ...look, color: c.id })}
          />
        ))}
      </div>
    </div>
  );
}

/** The first user-perceived character (an emoji with its modifiers counts as one); empty is none. */
function firstGlyph(text: string): string | null {
  const trimmed = text.trim();
  if (!trimmed) return null;
  const [first] = new Intl.Segmenter(undefined, { granularity: "grapheme" }).segment(trimmed);
  return first?.segment ?? null;
}
