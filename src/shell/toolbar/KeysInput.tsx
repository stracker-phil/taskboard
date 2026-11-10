import { useState, type KeyboardEvent } from "react";
import { api, toAppError, type Condition } from "../../api";
import { pickedValues, withValues } from "./conditionEditing";

interface Props {
  id: string;
  condition: Condition;
  onChange: (condition: Condition) => void;
  /** Why the typed keys were refused, or null once they are taken. */
  onError: (message: string | null) => void;
}

/** Issue keys typed as a list ("NW-123, NW-124"). The text applies on Enter or when the field loses focus, in
 *  the form Rust checked it. */
export function KeysInput({ id, condition, onChange, onError }: Props) {
  const stored = pickedValues(condition).join(", ");
  const [text, setText] = useState(stored);
  // A value changed elsewhere (a discarded draft, another field) replaces the typed text.
  const [shown, setShown] = useState(stored);
  if (stored !== shown) {
    setShown(stored);
    setText(stored);
  }

  const apply = async () => {
    if (text === stored) return;
    try {
      const keys = await api.filterParseKeys(text);
      onError(null);
      setText(keys.join(", "));
      onChange(withValues(condition, keys));
    } catch (e) {
      onError(toAppError(e).message);
    }
  };
  const onKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key !== "Enter") return;
    e.preventDefault();
    void apply();
  };

  return <input id={id} className="condition-row__value" type="text" placeholder="NW-123, NW-124" spellCheck={false} value={text} onChange={(e) => setText(e.target.value)} onBlur={() => void apply()} onKeyDown={onKeyDown} />;
}
