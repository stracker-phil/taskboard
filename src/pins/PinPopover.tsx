import { Pin } from "lucide-react";
import { useState, type RefObject } from "react";
import { api, type PinDuration, type PinKind } from "../api";
import { Popover } from "../components/Popover";
import { attempt } from "../state/requests";

const DURATIONS: { id: PinDuration; label: string }[] = [
  { id: "today", label: "Today" },
  { id: "twoDays", label: "2 days" },
  { id: "week", label: "1 week" },
  { id: "untilUnpinned", label: "Until unpinned" },
];

/** The mockup's preselected duration; ⌥-click pins with it too. */
export const DEFAULT_DURATION: PinDuration = "twoDays";

interface Props {
  title: string;
  /** Beside the title: who wrote the comment and when, or the issue key. */
  meta?: string;
  kind: PinKind;
  target: string;
  anchor: RefObject<HTMLElement | null>;
  onClose: () => void;
}

/** "Keep pinned for" and an optional reason; ↩ pins. */
export function PinPopover({ title, meta, kind, target, anchor, onClose }: Props) {
  const [duration, setDuration] = useState<PinDuration>(DEFAULT_DURATION);
  const [reason, setReason] = useState("");
  const pin = () => {
    void attempt(api.pinCreate(kind, target, duration, reason.trim() || null), "Could not pin it");
    onClose();
  };
  return (
    <Popover label={title} anchor={anchor} onClose={onClose} className="pin-popover">
      <form
        className="pin-popover__form"
        onSubmit={(e) => {
          e.preventDefault();
          pin();
        }}
        onKeyDown={(e) => {
          if (e.key !== "Enter") return;
          e.preventDefault();
          pin();
        }}
      >
        <header className="pin-popover__head">
          <Pin className="pin-popover__icon" size={16} strokeWidth={1.9} aria-hidden="true" />
          <span className="pin-popover__title">{title}</span>
          {meta && <span className="pin-popover__meta">{meta}</span>}
        </header>
        <span className="pin-popover__caps">Keep pinned for</span>
        <div className="pin-popover__chips" role="group" aria-label="Duration">
          {DURATIONS.map((d) => (
            <button key={d.id} type="button" className="pin-chip" aria-pressed={duration === d.id} onClick={() => setDuration(d.id)}>
              {d.label}
            </button>
          ))}
        </div>
        <label className="pin-popover__reason">
          <span className="pin-popover__caps">
            Reason <span className="pin-popover__hint">optional · shows in the sidebar</span>
          </span>
          <input value={reason} maxLength={200} onChange={(e) => setReason(e.target.value)} />
        </label>
        <div className="pin-popover__actions">
          <button type="button" className="btn-secondary" onClick={onClose}>
            Cancel
          </button>
          <button type="submit" className="btn-primary-dark">
            Pin
          </button>
        </div>
      </form>
    </Popover>
  );
}
