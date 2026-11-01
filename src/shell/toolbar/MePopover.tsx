import { Check } from "lucide-react";
import type { RefObject } from "react";
import type { MeRule, UserField } from "../../api";
import { Popover } from "../../components/Popover";
import { Toggle } from "../../components/Toggle";
import { ruleKey, ruleLabel } from "../../meMode/rules";

interface Props {
  rules: MeRule[];
  fields: UserField[];
  onRule: (rule: MeRule, on: boolean) => void;
  everywhere: boolean;
  onScope: (everywhere: boolean) => void;
  onAddFields: () => void;
  anchor: RefObject<HTMLElement | null>;
  onClose: () => void;
}

const SCOPES = [
  { everywhere: false, label: "Only this view", hint: "Saved with this view" },
  { everywhere: true, label: "All views", hint: "Everywhere, until you turn it off" },
];

/** What Me mode counts besides my assigned issues, and where it holds. Every switch applies at once. */
export function MePopover({ rules, fields, onRule, everywhere, onScope, onAddFields, anchor, onClose }: Props) {
  return (
    <Popover label="Me mode" anchor={anchor} onClose={onClose} className="me-popover">
      <div className="popover__header">
        <span className="popover__title">Me mode</span>
        <span className="popover__meta">Assigned to me, and</span>
      </div>
      <div className="me-popover__body">
        {rules.map((rule) => {
          const label = ruleLabel(rule.rule, fields);
          return (
            <button key={ruleKey(rule.rule)} type="button" className="column-row" aria-pressed={rule.on} onClick={() => onRule(rule, !rule.on)}>
              <span className="column-row__label">{label}</span>
              <Toggle on={rule.on} />
            </button>
          );
        })}
        <div className="popover__caps popover__caps--spaced">Applies to</div>
        {SCOPES.map((scope) => {
          const checked = scope.everywhere === everywhere;
          return (
            <button key={scope.label} type="button" aria-pressed={checked} className="me-popover__scope" onClick={() => onScope(scope.everywhere)}>
              <span className="me-popover__scope-text">
                <span className="column-row__label">{scope.label}</span>
                <span className="column-row__hint">{scope.hint}</span>
              </span>
              <Check size={14} strokeWidth={2.25} aria-hidden="true" className={checked ? "me-popover__check" : "me-popover__check me-popover__check--off"} />
            </button>
          );
        })}
      </div>
      <div className="popover__footer">
        <span className="popover__footer-text">Rules apply to every view</span>
        <button type="button" className="popover__small-btn" onClick={onAddFields}>
          Fields…
        </button>
      </div>
    </Popover>
  );
}
