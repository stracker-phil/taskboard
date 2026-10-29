import type { LucideIcon } from "lucide-react";
import { useState, type CSSProperties, type RefObject } from "react";
import { createPortal } from "react-dom";
import { Popover } from "../Popover";
import { LookEditor } from "./LookEditor";
import type { Look } from "./Tile";

interface Props {
  title: string;
  look: Look;
  fallback: string | LucideIcon;
  colorOptional?: boolean;
  anchor: RefObject<HTMLElement | null>;
  onChange: (look: Look) => void;
  onClose: () => void;
}

/** The look editor under the element it changes; on the window, so a scrolling sidebar doesn't clip it. */
export function LookPopover({ title, anchor, onClose, ...editor }: Props) {
  const [position] = useState(() => placeUnder(anchor.current));
  return createPortal(
    <Popover label={title} anchor={anchor} onClose={onClose} className="look-popover" style={position}>
      <span className="popover__caps">{title}</span>
      <LookEditor {...editor} />
    </Popover>,
    document.body,
  );
}

const HEIGHT = 300;
const GAP = 4;

/** Under the anchor, or above it when the window has no room underneath. */
function placeUnder(anchor: HTMLElement | null): CSSProperties {
  if (!anchor) return {};
  const rect = anchor.getBoundingClientRect();
  const below = rect.bottom + GAP + HEIGHT <= window.innerHeight;
  return below ? { left: rect.left, top: rect.bottom + GAP } : { left: rect.left, top: "auto", bottom: window.innerHeight - rect.top + GAP };
}
