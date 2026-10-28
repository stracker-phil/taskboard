import type { Style } from "../../api";
import { StatusRing } from "./StatusRing";

/** A status name in caps on its style's colour, with the ring inside. */
export function StatusPill({ style, name }: { style: Style; name: string }) {
  return (
    <span className={`status-pill status-pill--${style}`}>
      <StatusRing style={style} size={12} onPill />
      {name}
    </span>
  );
}
