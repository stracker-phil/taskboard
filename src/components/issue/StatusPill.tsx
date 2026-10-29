import type { Style } from "../../api";

/** `vertical`: the default pill turned 90°, for a collapsed Board column. */
export type PillVariant = "default" | "peek" | "activity" | "tab" | "vertical";

/** A status name in caps on its style's color; text only. */
export function StatusPill({ style, name, variant = "default" }: { style: Style; name: string; variant?: PillVariant }) {
  const className = ["status-pill", `status-pill--${style}`, variant !== "default" && `status-pill--${variant}`].filter(Boolean).join(" ");
  return <span className={className}>{name}</span>;
}
