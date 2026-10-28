import type { Style } from "../../api";

/** The circle of a status: dashed for To do, solid in the status colour otherwise. `onPill`: drawn inside a
 *  pill, in the pill's text colour. */
export function StatusRing({ style, size = 15, onPill = false }: { style: Style; size?: number; onPill?: boolean }) {
  const className = ["status-ring", `status-ring--${style}`, onPill && "status-ring--on-pill"].filter(Boolean).join(" ");
  return <span className={className} style={{ width: size, height: size }} aria-hidden="true" />;
}
