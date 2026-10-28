import { useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { isRecordingTarget } from "../commands/registry";

interface Props {
  onClose: () => void;
  /** `alertdialog` for a confirmation that interrupts. */
  role?: "dialog" | "alertdialog";
  labelledBy?: string;
  describedBy?: string;
  children: ReactNode;
}

const FOCUSABLE = 'a[href], button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])';

function focusables(root: HTMLElement): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>(FOCUSABLE)].filter((el) => !el.hidden);
}

/** A sheet over a scrim (Daybook's): Escape and a click on the scrim close it, Tab stays inside, and focus returns
 *  to the element that opened it. */
export function Modal({ onClose, role = "dialog", labelledBy, describedBy, children }: Props) {
  const sheetRef = useRef<HTMLDivElement>(null);
  const onCloseRef = useRef(onClose);
  useEffect(() => {
    onCloseRef.current = onClose;
  });
  // Read during the first render: children with autoFocus take focus before any effect runs.
  const [opener] = useState(() => document.activeElement as HTMLElement | null);

  useEffect(() => {
    const sheet = sheetRef.current;
    if (sheet && !sheet.contains(document.activeElement)) (focusables(sheet)[0] ?? sheet).focus();
    return () => {
      const active = document.activeElement;
      const focusLost = !active || active === document.body || !!sheet?.contains(active);
      if (focusLost && opener?.isConnected) opener.focus();
    };
  }, [opener]);

  useEffect(() => {
    // Capture phase, stopped here: the window's own Escape never also runs.
    const onKey = (e: globalThis.KeyboardEvent) => {
      if (e.key !== "Escape" || isRecordingTarget(e.target)) return;
      e.stopPropagation();
      onCloseRef.current();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, []);

  const trapTab = (e: KeyboardEvent) => {
    if (e.key !== "Tab" || !sheetRef.current) return;
    const items = focusables(sheetRef.current);
    if (items.length === 0) {
      e.preventDefault();
      return;
    }
    const first = items[0];
    const last = items[items.length - 1];
    const active = document.activeElement;
    if (e.shiftKey && (active === first || active === sheetRef.current)) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && active === last) {
      e.preventDefault();
      first.focus();
    }
  };

  return createPortal(
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div
        ref={sheetRef}
        className="dialog sheet"
        role={role}
        aria-modal="true"
        aria-labelledby={labelledBy}
        aria-describedby={describedBy}
        tabIndex={-1}
        onKeyDown={trapTab}
      >
        {children}
      </div>
    </div>,
    document.body,
  );
}
