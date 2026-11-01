import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type ReactNode, type RefObject } from "react";
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

/** The room a popover keeps from the edge of the panel it opens in. */
const EDGE = 8;

const FOCUSABLE = 'button:not(:disabled), input:not(:disabled), select:not(:disabled), [tabindex]:not([tabindex="-1"])';

/** A panel under its toolbar button. Escape and a press outside close it; focus moves in and goes back to the
 *  button when it closes. A popover that would reach past the content panel (or the window) opens toward the
 *  other side of its button instead, before it is drawn. */
export function Popover({ label, anchor, onClose, className, style, children }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const onCloseRef = useRef(onClose);
  useEffect(() => {
    onCloseRef.current = onClose;
  });
  const [opener] = useState(() => anchor.current);

  useLayoutEffect(() => {
    const panel = ref.current;
    if (!panel) return;
    const box = panel.getBoundingClientRect();
    const bounds = panel.parentElement?.closest(".content-panel")?.getBoundingClientRect() ?? { left: 0, right: window.innerWidth };
    // The opening animation scales it a little; its layout width tells where its far edge lands.
    const left = Math.min(box.left, box.right - panel.offsetWidth);
    const right = Math.max(box.right, box.left + panel.offsetWidth);
    if (left < bounds.left + EDGE) Object.assign(panel.style, { left: "0", right: "auto", transformOrigin: "top left" });
    else if (right > bounds.right - EDGE) Object.assign(panel.style, { left: "auto", right: "0", transformOrigin: "top right" });
  }, []);

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
