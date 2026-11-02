import { api, type MeMatch, type MeRule } from "../../api";
import { useModes } from "../../queries/modes";
import { useSettings } from "../../queries/settings";
import { useMeUserFields } from "../../queries/views";
import { attempt } from "../../state/requests";
import { useStore } from "../../state/store";
import { onOf, rulesFor } from "./meRules";

/** Me mode in one view (ADR 0045): on when the view has it (saved or drafted) or when it holds in every view. Me and
 *  the person chip exclude each other. A view's own Me mode has its own rules (ADR 0051). */
export function useMeMode(key: string, viewMe: boolean, viewPerson: string | null, viewMeRules: MeMatch[] | null) {
  const modes = useModes();
  const { data: settings } = useSettings();
  const { data: fields = [] } = useMeUserFields();
  const setNarrowing = useStore((s) => s.setNarrowing);
  const everywhere = modes.meEverywhere;
  const on = viewMe || everywhere;

  const setEverywhere = (next: boolean) => void attempt(api.modeSet("meEverywhere", next), "Could not change where Me mode applies");
  const setMe = (me: boolean) => {
    if (!me && everywhere) setEverywhere(false);
    setNarrowing(key, me ? { me, person: null } : { me });
  };
  const setPerson = (person: string | null) => {
    if (person && everywhere) setEverywhere(false);
    setNarrowing(key, { person, me: person ? false : viewMe });
  };
  const setScope = (all: boolean) => {
    setEverywhere(all);
    setNarrowing(key, all ? { person: null } : { me: true, person: null });
  };
  const appRules = settings?.behaviour.meRules ?? [];
  const usesViewRules = viewMe || !everywhere;
  const rules = usesViewRules ? rulesFor(appRules, viewMeRules) : appRules;
  const setRule = (rule: MeRule, ruleOn: boolean) => {
    if (!usesViewRules) {
      void attempt(api.meRuleEdit({ kind: "switch", rule: rule.rule, on: ruleOn }), "Could not change Me mode");
      return;
    }
    const next = rules.map((r) => (r.rule === rule.rule ? { ...r, on: ruleOn } : r));
    setNarrowing(key, { meRules: onOf(next) });
  };

  return {
    on,
    everywhere,
    person: viewPerson,
    rules,
    fields,
    setMe,
    toggle: () => setMe(!on),
    setPerson,
    setScope,
    setRule,
  };
}
