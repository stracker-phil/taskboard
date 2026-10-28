/** A switch drawn inside a button that carries `aria-pressed`. */
export function Toggle({ on }: { on: boolean }) {
  return (
    <span className={on ? "toggle toggle--on" : "toggle"} aria-hidden="true">
      <span className="toggle__knob" />
    </span>
  );
}
