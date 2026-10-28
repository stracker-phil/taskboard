/** Where a dragged row would land; `y` from the top of its list, nothing while no drag runs. */
export function DropLine({ y }: { y: number | null }) {
  return y === null ? null : <div className="sb-drop-line" style={{ top: y }} aria-hidden="true" />;
}
