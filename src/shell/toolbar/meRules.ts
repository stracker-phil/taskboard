import type { MeMatch, MeRule } from "../../api";

const sameMatch = (a: MeMatch, b: MeMatch) => a.kind === b.kind && (a.kind !== "field" || (b.kind === "field" && a.field === b.field));

/** The app's rules (Settings → Me Mode) switched as one view has them: `on` lists the rules on; null keeps the
 *  app's switches (ADR 0051). */
export function rulesFor(app: MeRule[], on: MeMatch[] | null): MeRule[] {
  return on === null ? app : app.map((rule) => ({ rule: rule.rule, on: on.some((match) => sameMatch(match, rule.rule)) }));
}

/** The rules switched on, in the app's order. */
export const onOf = (rules: MeRule[]): MeMatch[] => rules.filter((rule) => rule.on).map((rule) => rule.rule);

/** Both switch on the same rules of the app's. */
export const sameOn = (app: MeRule[], a: MeMatch[] | null, b: MeMatch[] | null) =>
  rulesFor(app, a).every((rule, i) => rule.on === rulesFor(app, b)[i].on);
