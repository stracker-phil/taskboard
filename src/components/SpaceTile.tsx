import type { SpaceNode } from "../api";
import { Tile } from "./look/Tile";

/** A Space's tile: its icon (else the name's first letter) on its color (else the first): 22 in the sidebar, 28
 *  in the content header, 20 in the breadcrumb. */
export function SpaceTile({ space, size }: { space: SpaceNode; size: 20 | 22 | 28 }) {
  return <Tile icon={space.icon} color={space.color ?? "1"} fallback={space.name.charAt(0)} size={size} />;
}
