import type { Style } from "../../api";

export type PillVariant = "default" | "peek" | "activity" | "tab";

/** A status name in caps on its style's colour; text only. */
export function StatusPill({ style, name, variant = "default" }: { style: Style; name: string; variant?: PillVariant }) {
  const className = ["status-pill", `status-pill--${style}`, variant !== "default" && `status-pill--${variant}`].filter(Boolean).join(" ");
  return <span className={className}>{name}</span>;
}
