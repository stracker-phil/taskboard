import { Bookmark } from "lucide-react";
import { useRef, useState, type MouseEvent } from "react";
import { api, type PinKind } from "../api";
import { usePinOf } from "../queries/pins";
import { attempt } from "../state/requests";
import { PinPopover, usePinDefault } from "./PinPopover";

interface Props {
  kind: PinKind;
  target: string;
  /** The popover's title and the button's name ("Bookmark this issue", "Bookmark comment"). */
  label: string;
  meta?: string;
  className: string;
  iconSize?: number;
}

/** Opens the bookmark popover; ⌥-click bookmarks at once, and a bookmarked target loses its bookmark on click. */
export function PinButton({ kind, target, label, meta, className, iconSize = 14 }: Props) {
  const pin = usePinOf(kind, target);
  const pinDefault = usePinDefault();
  const [open, setOpen] = useState(false);
  const anchor = useRef<HTMLButtonElement>(null);
  const press = (e: MouseEvent) => {
    e.stopPropagation();
    if (pin) void attempt(api.pinRemove(pin.id), "Could not remove the bookmark");
    else if (e.altKey) void attempt(api.pinCreate(kind, target, pinDefault, null), "Could not bookmark it");
    else setOpen((was) => !was);
  };
  return (
    <span className="pin-anchor">
      <button
        ref={anchor}
        type="button"
        className={className}
        aria-label={pin ? "Remove bookmark" : label}
        title={pin ? "Remove bookmark" : `${label} (⌥-click: bookmark at once)`}
        aria-pressed={pin !== undefined}
        aria-expanded={open}
        onClick={press}
      >
        <Bookmark size={iconSize} strokeWidth={1.9} aria-hidden="true" />
      </button>
      {open && <PinPopover title={label} meta={meta} kind={kind} target={target} anchor={anchor} onClose={() => setOpen(false)} />}
    </span>
  );
}
