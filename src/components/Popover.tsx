import { useEffect, useRef, useState, type CSSProperties, type ReactNode, type RefObject } from "react";
import { isRecordingTarget } from "../commands/registry";

interface Props {
  label: string;
  /** The button that opens it: a press on it is left to its own toggle, and focus returns to it. */
  anchor: RefObject<HTMLElement | null>;
  onClose: () => void;
  className: string;
  style?: CSSProperties;
  children: ReactNode;
}

const FOCUSABLE = 'button:not(:disabled), input:not(:disabled), select:not(:disabled), [tabindex]:not([tabindex="-1"])';

/** A panel under its toolbar button. Escape and a press outside close it; focus moves in and goes back to the
 *  button when it closes. */
export function Popover({ label, anchor, onClose, className, style, children }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const onCloseRef = useRef(onClose);
  useEffect(() => {
    onCloseRef.current = onClose;
  });
  const [opener] = useState(() => anchor.current);

  useEffect(() => {
    const panel = ref.current;
    panel?.querySelector<HTMLElement>(FOCUSABLE)?.focus();
    return () => {
      const active = document.activeElement;
      const focusLost = !active || active === document.body || !!panel?.contains(active);
      if (focusLost && opener?.isConnected) opener.focus();
    };
  }, [opener]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || isRecordingTarget(e.target)) return;
      e.stopPropagation();
      onCloseRef.current();
    };
    const onPress = (e: MouseEvent) => {
      const target = e.target as Node;
      if (ref.current?.contains(target) || anchor.current?.contains(target)) return;
      onCloseRef.current();
    };
    window.addEventListener("keydown", onKey, true);
    document.addEventListener("mousedown", onPress, true);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      document.removeEventListener("mousedown", onPress, true);
    };
  }, [anchor]);

  return (
    <div ref={ref} className={`popover ${className}`} style={style} role="dialog" aria-label={label}>
      {children}
    </div>
  );
}
