import type { RefObject } from "react";
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
  return (
    <Popover label="Icon" anchor={anchor} onClose={onClose} className="icon-picker">
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
    </Popover>
  );
}
