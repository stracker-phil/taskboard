import { useState, type CSSProperties, type RefObject } from "react";
import { createPortal } from "react-dom";
import { Popover } from "./Popover";
import { LIST_ICONS } from "./listIcons";

interface Props {
  /** The current icon's name; none is the funnel. */
  value: string | null;
  anchor: RefObject<HTMLElement | null>;
  onPick: (name: string | null) => void;
  onClose: () => void;
}

/** A grid of the list icons (ClickUp style); the funnel resets to the default. */
export function IconPicker({ value, anchor, onPick, onClose }: Props) {
  const current = value && value in LIST_ICONS ? value : "funnel";
  const [position] = useState(() => placeUnder(anchor.current));
  // On the window, so the sidebar's scrolling doesn't clip it.
  return createPortal(
    <Popover label="Icon" anchor={anchor} onClose={onClose} className="icon-picker" style={position}>
      <span className="popover__caps">Icon</span>
      <div className="icon-picker__grid" role="radiogroup" aria-label="Icon">
        {Object.entries(LIST_ICONS).map(([name, Icon]) => (
          <button
            key={name}
            type="button"
            role="radio"
            aria-checked={name === current}
            aria-label={name.replace(/-/g, " ")}
            className="icon-picker__option"
            onClick={() => {
              onPick(name === "funnel" ? null : name);
              onClose();
            }}
          >
            <Icon size={16} strokeWidth={1.9} aria-hidden="true" />
          </button>
        ))}
      </div>
    </Popover>,
    document.body,
  );
}

const PICKER_HEIGHT = 210;
const GAP = 4;

/** Under the anchor, or above it when the window has no room underneath. */
function placeUnder(anchor: HTMLElement | null): CSSProperties {
  if (!anchor) return {};
  const rect = anchor.getBoundingClientRect();
  const below = rect.bottom + GAP + PICKER_HEIGHT <= window.innerHeight;
  return below ? { left: rect.left, top: rect.bottom + GAP } : { left: rect.left, top: "auto", bottom: window.innerHeight - rect.top + GAP };
}
