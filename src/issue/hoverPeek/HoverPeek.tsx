import { createPortal } from "react-dom";
import { PeekCard, place } from "./PeekCard";
import { useHoverPeek } from "./useHoverPeek";

/** ⌥ + hover over any issue: a small card with its summary, status, people and description, without leaving
 *  the place. It never takes the pointer, so the hovered element keeps it. */
export function HoverPeek() {
  const target = useHoverPeek();
  return target ? createPortal(<PeekCard id={target.id} style={place(target.rect)} />, document.body) : null;
}
