import type { Style } from "../../api";

/** The one status marker: a filled 10px square in the style's color. */
export function StatusSquare({ style }: { style: Style }) {
  return <span className={`status-square status-square--${style}`} aria-hidden="true" />;
}
