import { Pin } from "lucide-react";
import { useRef, useState, type MouseEvent } from "react";
import { api, type PinKind } from "../api";
import { usePinOf } from "../queries/pins";
import { attempt } from "../state/requests";
import { DEFAULT_DURATION, PinPopover } from "./PinPopover";

interface Props {
  kind: PinKind;
  target: string;
  /** The popover's title and the button's name ("Pin this issue", "Pin comment"). */
  label: string;
  meta?: string;
  className: string;
  iconSize?: number;
}

/** Opens the pin popover; ⌥-click pins at once, and a pinned target unpins on click. */
export function PinButton({ kind, target, label, meta, className, iconSize = 14 }: Props) {
  const pin = usePinOf(kind, target);
  const [open, setOpen] = useState(false);
  const anchor = useRef<HTMLButtonElement>(null);
  const press = (e: MouseEvent) => {
    e.stopPropagation();
    if (pin) void attempt(api.pinRemove(pin.id), "Could not unpin it");
    else if (e.altKey) void attempt(api.pinCreate(kind, target, DEFAULT_DURATION, null), "Could not pin it");
    else setOpen((was) => !was);
  };
  return (
    <span className="pin-anchor">
      <button
        ref={anchor}
        type="button"
        className={className}
        aria-label={pin ? "Unpin" : label}
        title={pin ? "Unpin" : `${label} (⌥-click: pin at once)`}
        aria-pressed={pin !== undefined}
        aria-expanded={open}
        onClick={press}
      >
        <Pin size={iconSize} strokeWidth={1.9} aria-hidden="true" />
      </button>
      {open && <PinPopover title={label} meta={meta} kind={kind} target={target} anchor={anchor} onClose={() => setOpen(false)} />}
    </span>
  );
}
