import { X } from "lucide-react";
import { useId } from "react";
import type { Condition, FilterField, FilterOptions, Op } from "../../api";
import { choicesFor, DEFAULT_DAYS, newCondition, OP_NAMES, opsFor, startValue, takesDays, takesNoValue } from "./conditionEditing";

interface Props {
  condition: Condition;
  options: FilterOptions;
  onChange: (condition: Condition) => void;
  onRemove: () => void;
}

/** One own condition as field / operator / value, with a remove button. */
export function ConditionRow({ condition, options, onChange, onRemove }: Props) {
  const id = useId();
  const field: FilterField = options.fields.find((f) => f.id === condition.field) ?? { id: condition.field, name: condition.field, kind: "text" };
  const ops = opsFor(field.kind);

  const setField = (fieldId: string) => {
    const next = options.fields.find((f) => f.id === fieldId);
    if (next) onChange(newCondition(next, options));
  };
  const setOp = (op: Op) => {
    const keepsValue = takesDays(op) === takesDays(condition.op) && !takesNoValue(condition.op);
    let value = keepsValue ? condition.value : startValue(field, op, options);
    if (takesNoValue(op)) value = undefined;
    onChange({ ...condition, op, value });
  };

  return (
    <div className="condition-row">
      <label className="visually-hidden" htmlFor={`${id}-field`}>
        Field
      </label>
      <select id={`${id}-field`} className="condition-row__field" value={field.id} onChange={(e) => setField(e.target.value)}>
        {options.fields.map((f) => (
          <option key={f.id} value={f.id}>
            {f.name}
          </option>
        ))}
      </select>
      <label className="visually-hidden" htmlFor={`${id}-op`}>
        Operator
      </label>
      <select id={`${id}-op`} className="condition-row__op" value={condition.op} onChange={(e) => setOp(e.target.value as Op)}>
        {ops.map((op) => (
          <option key={op} value={op}>
            {OP_NAMES[op]}
          </option>
        ))}
      </select>
      <ValueEditor id={`${id}-value`} condition={condition} field={field} options={options} onChange={onChange} />
      <button type="button" className="condition-row__remove" aria-label="Remove condition" onClick={onRemove}>
        <X size={14} aria-hidden="true" />
      </button>
    </div>
  );
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
        <select id={id} className="condition-row__value" value={text} onChange={(e) => onChange({ ...condition, value: e.target.value })}>
          {choices.map((c) => (
            <option key={c.id} value={c.id}>
              {c.name}
            </option>
          ))}
        </select>
      </>
    );
  }
  return (
    <>
      {label}
      <input id={id} className="condition-row__value" type={field.kind === "date" ? "date" : "text"} value={text} onChange={(e) => onChange({ ...condition, value: e.target.value })} />
    </>
  );
}
