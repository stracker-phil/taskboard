/** Shared controls for settings pages: inset groups of rows that apply their change right away. */
import { createContext, useContext, useEffect, useId, useRef, type ReactNode } from "react";
import { SegmentedControl, type SegmentedControlProps } from "../SegmentedControl";

/** The row or group Settings opens at; a new object per request, so opening it again reveals it again. */
export interface RevealTarget {
  id: string;
  /** Revealed instead while the row isn't shown. */
  under?: string;
}

export const ScrollTarget = createContext<RevealTarget | undefined>(undefined);

interface RevealState {
  revealed: boolean;
  fallback?: HTMLElement;
}

/** Per request: whether its row was revealed, and the `under` element that waits in case it isn't shown. */
const reveals = new WeakMap<RevealTarget, RevealState>();

function scrollAndHighlight(element: HTMLElement) {
  element.scrollIntoView?.({ block: "start" });
  element.classList.remove("found");
  void element.offsetWidth; // restarts the highlight
  element.classList.add("found");
}

/** Scrolls to and washes the element whose `anchor` the request names; its `under` element only while the row
 *  itself doesn't show. */
function useScrollTarget<E extends HTMLElement>(anchor: string | undefined) {
  const target = useContext(ScrollTarget);
  const ref = useRef<E>(null);
  useEffect(() => {
    const element = ref.current;
    if (!anchor || !target || !element) return;
    const state = reveals.get(target) ?? { revealed: false };
    reveals.set(target, state);
    if (state.revealed) return;
    if (anchor === target.id) {
      state.revealed = true;
      state.fallback?.classList.remove("found");
      scrollAndHighlight(element);
    } else if (anchor === target.under && !state.fallback) {
      state.fallback = element;
      // After this render's effects: the row itself may be shown further down and take the highlight.
      queueMicrotask(() => {
        if (!state.revealed) scrollAndHighlight(element);
      });
    }
  }, [anchor, target]);
  return ref;
}

/** An inset group with its heading above it. `anchor`: its id in the page's `rows` (`pages.tsx`). */
export function Group({ title, description, anchor, children }: { title?: string; description?: ReactNode; anchor?: string; children: ReactNode }) {
  const ref = useScrollTarget<HTMLElement>(anchor);
  return (
    <section ref={ref} className="settings-group" aria-label={title} data-anchor={anchor}>
      {title && <h3>{title}</h3>}
      {description && <p className="settings-group-desc">{description}</p>}
      <div className="settings-card">{children}</div>
    </section>
  );
}

interface RowProps {
  label: ReactNode;
  description?: ReactNode;
  /** Its id in the page's `rows` (`pages.tsx`); data rows (a project, a status) have none. */
  anchor?: string;
  htmlFor?: string;
  /** The control goes below the label instead of beside it. */
  stacked?: boolean;
  /** Dimmed: it has no effect with the current settings. */
  disabled?: boolean;
  children?: ReactNode;
}

/** One group row: label and description on the left, the control (a value, a `.btn`, a switch) on the right. */
export function Row({ label, description, anchor, htmlFor, stacked, disabled, children }: RowProps) {
  const ref = useScrollTarget<HTMLDivElement>(anchor);
  const className = ["settings-row", stacked && "stacked", disabled && "disabled"].filter(Boolean).join(" ");
  return (
    <div ref={ref} className={className} data-anchor={anchor}>
      <div className="settings-row-text">
        {htmlFor ? <label htmlFor={htmlFor}>{label}</label> : <div className="settings-row-label">{label}</div>}
        {description && <div className="settings-row-desc">{description}</div>}
      </div>
      <div className="settings-row-control">{children}</div>
    </div>
  );
}

/** A bare switch; its label comes from the caller (a `<label>`, `htmlFor` or `aria-label`). */
export function SwitchInput({ checked, onChange, ...input }: { id?: string; "aria-label"?: string; disabled?: boolean; checked: boolean; onChange: (on: boolean) => void }) {
  return <input {...input} className="switch" type="checkbox" role="switch" checked={checked} onChange={(e) => onChange(e.target.checked)} />;
}

/** A row whose control is a switch. */
export function Switch({ label, description, anchor, checked, onChange }: { label: string; description?: ReactNode; anchor?: string; checked: boolean; onChange: (on: boolean) => void }) {
  const id = useId();
  return (
    <Row label={label} description={description} anchor={anchor} htmlFor={id}>
      <SwitchInput id={id} checked={checked} onChange={onChange} />
    </Row>
  );
}

/** Controls without a label, e.g. a form's buttons and its result. */
export function ControlRow({ children }: { children: ReactNode }) {
  return <div className="settings-row control-row">{children}</div>;
}

/** A row whose control is a segmented control. */
export function Segmented<T extends string>({ label, description, anchor, ...buttons }: SegmentedControlProps<T> & { description?: ReactNode; anchor?: string }) {
  return (
    <Row label={label} description={description} anchor={anchor} disabled={buttons.disabled}>
      <SegmentedControl label={label} {...buttons} />
    </Row>
  );
}
