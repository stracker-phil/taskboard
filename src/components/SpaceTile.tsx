import type { SpaceNode } from "../api";

/** The owner's icon letter, else the name's first letter. */
const letter = (space: SpaceNode) => (space.icon ?? space.name.charAt(0)).toUpperCase();

/** A Space's letter on its colour token (`--space-tile-N`; the first without one): 22 in the sidebar, 28 in the
 *  content header, 20 in the breadcrumb. */
export function SpaceTile({ space, size }: { space: SpaceNode; size: 20 | 22 | 28 }) {
  return (
    <span className={`space-tile space-tile--${size}`} style={{ background: `var(--space-tile-${space.color ?? "1"})` }} aria-hidden="true">
      {letter(space)}
    </span>
  );
}
