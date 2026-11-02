import { X } from "lucide-react";
import { useId } from "react";
import type { Condition, FilterField, FilterOptions, Op } from "../../api";
import { choicesFor, DEFAULT_DAYS, newCondition, OP_NAMES, opsFor, pickedValues, shownOp, startValue, takesDays, takesNoValue, withValues } from "./conditionEditing";
import { ValuePicker } from "./ValuePicker";

interface Props {
  condition: Condition;
  options: FilterOptions;
  onChange: (condition: Condition) => void;
  /** None for a condition that can't be removed: its field is fixed too, and only `ops` are offered. */
  onRemove?: () => void;
  ops?: Op[];
}

/** One own condition as field / operator / value, with a remove button unless it is fixed. */
export function ConditionRow({ condition, options, onChange, onRemove, ops: fixedOps }: Props) {
  const id = useId();
  const field: FilterField = options.fields.find((f) => f.id === condition.field) ?? { id: condition.field, name: condition.field, kind: "text" };
  const ops = fixedOps ?? opsFor(field.kind);

  const setField = (fieldId: string) => {
    const next = options.fields.find((f) => f.id === fieldId);
    if (next) onChange(newCondition(next, options));
  };
  const setOp = (op: Op) => {
    // Picked values survive switching between "is" and "is not".
    if ((op === "is" || op === "isNot") && choicesFor(field.kind, options) && !takesNoValue(condition.op)) {
      onChange(withValues({ ...condition, op }, pickedValues(condition)));
      return;
    }
    const keepsValue = takesDays(op) === takesDays(condition.op) && !takesNoValue(condition.op);
    let value = keepsValue ? countIn(op, condition) : startValue(field, op, options);
    if (takesNoValue(op)) value = undefined;
    onChange({ ...condition, op, value });
  };

  return (
    <div className="condition-row">
      <label className="visually-hidden" htmlFor={`${id}-field`}>
        Field
      </label>
      <select id={`${id}-field`} className="condition-row__field" value={field.id} disabled={!onRemove} onChange={(e) => setField(e.target.value)}>
        {options.fields.map((f) => (
          <option key={f.id} value={f.id}>
            {f.name}
          </option>
        ))}
      </select>
      <label className="visually-hidden" htmlFor={`${id}-op`}>
        Operator
      </label>
      <select id={`${id}-op`} className="condition-row__op" value={shownOp(condition.op)} onChange={(e) => setOp(e.target.value as Op)}>
        {ops.map((op) => (
          <option key={op} value={op}>
            {OP_NAMES[op]}
          </option>
        ))}
      </select>
      <ValueEditor id={`${id}-value`} condition={condition} field={field} options={options} onChange={onChange} />
      {onRemove ? (
        <button type="button" className="condition-row__remove" aria-label="Remove condition" onClick={onRemove}>
          <X size={14} aria-hidden="true" />
        </button>
      ) : (
        // Keeps the value as wide as in the other rows.
        <span className="condition-row__remove" aria-hidden="true" />
      )}
    </div>
  );
}

/** A count moving between hours and days keeps its span: 24 hours become 1 day, 2 days 48 hours. */
function countIn(op: Op, condition: Condition): Condition["value"] {
  const count = condition.value;
  if (typeof count !== "number") return count;
  const hours = (o: Op) => o === "withinLastHours";
  if (hours(condition.op) && !hours(op)) return Math.max(1, Math.round(count / 24));
  if (!hours(condition.op) && hours(op)) return count * 24;
  return count;
}

function ValueEditor({ id, condition, field, options, onChange }: { id: string; condition: Condition; field: FilterField; options: FilterOptions; onChange: (c: Condition) => void }) {
  const label = (
    <label className="visually-hidden" htmlFor={id}>
      Value
    </label>
  );
  if (takesNoValue(condition.op)) return <span className="condition-row__value" />;
  if (takesDays(condition.op)) {
    return (
      <>
        {label}
        <input id={id} className="condition-row__value" type="number" min={1} value={typeof condition.value === "number" ? condition.value : DEFAULT_DAYS} onChange={(e) => onChange({ ...condition, value: Number(e.target.value) })} />
      </>
    );
  }
  const text = typeof condition.value === "string" ? condition.value : "";
  const choices = choicesFor(field.kind, options);
  if (choices) {
    return (
      <>
        {label}
        <ValuePicker id={id} choices={choices} selected={pickedValues(condition)} onChange={(values) => onChange(withValues(condition, values))} />
      </>
    );
  }
  return (
    <>
      {label}
      <input id={id} className="condition-row__value" type={field.kind === "date" || field.kind === "timestamp" ? "date" : "text"} value={text} onChange={(e) => onChange({ ...condition, value: e.target.value })} />
    </>
  );
}
