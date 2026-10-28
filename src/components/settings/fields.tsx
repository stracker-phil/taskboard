/** Text, number and date fields for settings rows, each showing the error Rust gave for its field. */
import { useEffect, useId, useRef, useState, type InputHTMLAttributes, type ReactNode } from "react";
import { Row } from "./controls";

type FieldType = "text" | "number" | "date" | "email" | "password" | "url";

type BufferedInputProps = Omit<InputHTMLAttributes<HTMLInputElement>, "value" | "type" | "min" | "max" | "required"> & {
  value: string;
  onCommit: (v: string) => void;
  type?: "text" | "number" | "date";
  min?: number;
  max?: number;
  /** An empty value isn't saved; the field shows the saved one again. */
  required?: boolean;
};

/** Input that commits on blur or Enter, so half-typed values never reach the backend.
 *  Number spinner steps (input without typing) also commit, debounced. */
function BufferedInput({ value, onCommit, type = "text", min, max, required, ...input }: BufferedInputProps) {
  const [text, setText] = useState(value);
  const stepTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  useEffect(() => setText(value), [value]);
  useEffect(() => () => clearTimeout(stepTimer.current), []);

  const isValid = (draft: string) => {
    if (required && !draft.trim()) return false;
    if (type !== "number") return true;
    const n = Number(draft);
    return draft.trim() !== "" && Number.isInteger(n) && n >= (min ?? -Infinity) && n <= (max ?? Infinity);
  };
  const commit = (draft: string) => {
    clearTimeout(stepTimer.current);
    if (draft === value) return;
    if (isValid(draft)) onCommit(draft);
    else setText(value);
  };

  return (
    <input
      {...input}
      type={type}
      min={min}
      max={max}
      value={text}
      onChange={(e) => {
        const next = e.target.value;
        setText(next);
        // Typing reports an inputType; spinner clicks and arrow-key steps do not.
        const typed = !!(e.nativeEvent as InputEvent).inputType;
        if (type !== "number" || typed || !isValid(next)) return;
        clearTimeout(stepTimer.current);
        stepTimer.current = setTimeout(() => commit(next), 300);
      }}
      onBlur={() => commit(text)}
      onKeyDown={(e) => e.key === "Enter" && commit(text)}
    />
  );
}

export function FieldError({ id, message }: { id: string; message?: string | null }) {
  if (!message) return null;
  return (
    <span id={id} className="settings-field-error">
      {message}
    </span>
  );
}

const inputClass = (type: FieldType, wide?: boolean, mono?: boolean) =>
  ["input", type === "number" && "number", wide && "wide", mono && "mono"].filter(Boolean).join(" ");

interface FieldProps {
  label: string;
  description?: ReactNode;
  placeholder?: string;
  /** Room for a URL. */
  wide?: boolean;
  /** Rust's message for this field. */
  error?: string | null;
  suffix?: string;
}

/** A row with an input and its error below it; `children` gets the input's id and its error's id. */
function FieldRow({ label, description, error, suffix, children }: FieldProps & { children: (ids: { id: string; errorId: string }) => ReactNode }) {
  const id = useId();
  const errorId = `${id}-error`;
  return (
    <Row label={label} description={description} htmlFor={id}>
      <div className="settings-field">
        <div className="settings-input">
          {children({ id, errorId })}
          {suffix && <span className="settings-suffix">{suffix}</span>}
        </div>
        <FieldError id={errorId} message={error} />
      </div>
    </Row>
  );
}

export function TextField({ value, onCommit, type = "text", min, max, required, ...row }: FieldProps & {
  value: string;
  onCommit: (v: string) => void;
  type?: "text" | "number" | "date";
  min?: number;
  max?: number;
  required?: boolean;
}) {
  return (
    <FieldRow {...row}>
      {({ id, errorId }) => (
        <BufferedInput
          id={id}
          className={inputClass(type, row.wide)}
          type={type}
          min={min}
          max={max}
          required={required}
          placeholder={row.placeholder}
          aria-invalid={row.error ? true : undefined}
          aria-describedby={row.error ? errorId : undefined}
          value={value}
          onCommit={onCommit}
        />
      )}
    </FieldRow>
  );
}

/** Commits only a whole number within `min` and `max`. */
export function NumberField({ value, onCommit, ...rest }: FieldProps & { value: number; onCommit: (v: number) => void; min: number; max: number }) {
  return <TextField {...rest} type="number" value={String(value)} onCommit={(text) => onCommit(Number(text))} />;
}

/** A field of a form that saves with its Save button: every keystroke goes to `onChange`. */
export function DraftField({ value, onChange, type = "text", mono, autoComplete, ...row }: FieldProps & {
  value: string;
  onChange: (v: string) => void;
  type?: FieldType;
  mono?: boolean;
  autoComplete?: string;
}) {
  return (
    <FieldRow {...row}>
      {({ id, errorId }) => (
        <input
          id={id}
          className={inputClass(type, row.wide, mono)}
          type={type}
          placeholder={row.placeholder}
          autoComplete={autoComplete}
          spellCheck={false}
          aria-invalid={row.error ? true : undefined}
          aria-describedby={row.error ? errorId : undefined}
          value={value}
          onChange={(e) => onChange(e.target.value)}
        />
      )}
    </FieldRow>
  );
}
