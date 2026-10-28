/** Shared controls for settings pages: inset groups of rows that apply their change right away. */
import { createContext, useContext, useEffect, useId, useRef, type ReactNode } from "react";
import { SegmentedControl, type SegmentedControlProps } from "../SegmentedControl";

/** The section `open_settings` asked for; its group scrolls into view. */
export const SectionTarget = createContext<string | undefined>(undefined);

/** An inset group with its heading above it. `anchor`: the section id (`registerSettingsPage` `sections`) that
 *  scrolls to this group. */
export function Group({ title, description, anchor, children }: { title?: string; description?: ReactNode; anchor?: string; children: ReactNode }) {
  const target = useContext(SectionTarget);
  const ref = useRef<HTMLElement>(null);
  useEffect(() => {
    if (anchor && anchor === target) ref.current?.scrollIntoView?.({ block: "start" });
  }, [anchor, target]);
  return (
    <section ref={ref} className="settings-group" aria-label={title} data-section={anchor}>
      {title && <h3>{title}</h3>}
      {description && <p className="settings-group-desc">{description}</p>}
      <div className="settings-card">{children}</div>
    </section>
  );
}

interface RowProps {
  label: string;
  description?: ReactNode;
  htmlFor?: string;
  /** The control goes below the label instead of beside it. */
  stacked?: boolean;
  /** Dimmed: it has no effect with the current settings. */
  disabled?: boolean;
  children?: ReactNode;
}

/** One group row: label and description on the left, the control (a value, a `.btn`, a switch) on the right. */
export function Row({ label, description, htmlFor, stacked, disabled, children }: RowProps) {
  const className = ["settings-row", stacked && "stacked", disabled && "disabled"].filter(Boolean).join(" ");
  return (
    <div className={className}>
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
export function Switch({ label, description, checked, onChange }: { label: string; description?: ReactNode; checked: boolean; onChange: (on: boolean) => void }) {
  const id = useId();
  return (
    <Row label={label} description={description} htmlFor={id}>
      <SwitchInput id={id} checked={checked} onChange={onChange} />
    </Row>
  );
}

/** Controls without a label, e.g. a form's buttons and its result. */
export function ControlRow({ children }: { children: ReactNode }) {
  return <div className="settings-row control-row">{children}</div>;
}

/** A row whose control is a segmented control. */
export function Segmented<T extends string>({ label, description, ...buttons }: SegmentedControlProps<T> & { description?: ReactNode }) {
  return (
    <Row label={label} description={description} disabled={buttons.disabled}>
      <SegmentedControl label={label} {...buttons} />
    </Row>
  );
}
