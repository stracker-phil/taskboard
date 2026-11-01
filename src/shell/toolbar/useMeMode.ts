import { api, type MeRule } from "../../api";
import { useModes } from "../../queries/modes";
import { useSettings } from "../../queries/settings";
import { useMeUserFields } from "../../queries/views";
import { attempt } from "../../state/requests";
import { NO_VIEW_NARROWING } from "../../state/slices/narrowing";
import { useStore } from "../../state/store";

/** Me mode in one view (ADR 0045): on when the view has it (saved or drafted) or when it holds in every view.
 *  Me and the person chip exclude each other. */
export function useMeMode(key: string, viewMe: boolean) {
  const modes = useModes();
  const { data: settings } = useSettings();
  const { data: fields = [] } = useMeUserFields();
  const narrowing = useStore((s) => s.narrowings[key]) ?? NO_VIEW_NARROWING;
  const setNarrowing = useStore((s) => s.setNarrowing);
  const everywhere = modes.meEverywhere;
  const on = viewMe || everywhere;

  const setEverywhere = (next: boolean) => void attempt(api.modeSet("meEverywhere", next), "Could not change where Me mode applies");
  const setMe = (me: boolean) => {
    if (!me && everywhere) setEverywhere(false);
    setNarrowing(key, { me, person: me ? null : narrowing.person });
  };
  const setPerson = (person: string | null) => {
    if (person && everywhere) setEverywhere(false);
    setNarrowing(key, { person, me: person ? false : viewMe });
  };
  const setScope = (all: boolean) => {
    setEverywhere(all);
    setNarrowing(key, all ? { person: null } : { me: true, person: null });
  };
  const setRule = (rule: MeRule, ruleOn: boolean) =>
    void attempt(api.meRuleEdit({ kind: "switch", rule: rule.rule, on: ruleOn }), "Could not change Me mode");

  return {
    on,
    everywhere,
    person: narrowing.person,
    rules: settings?.behaviour.meRules ?? [],
    fields,
    setMe,
    toggle: () => setMe(!on),
    setPerson,
    setScope,
    setRule,
  };
}
