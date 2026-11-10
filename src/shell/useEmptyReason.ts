import { useQuery } from "@tanstack/react-query";
import { api, type Narrowing } from "../api";
import { useViews } from "../queries/sidebar";
import { useListConditions, useViewPeople } from "../queries/views";
import { isComplete } from "./toolbar/conditionEditing";

interface MeNarrowing {
  on: boolean;
  /** The person chip's account. */
  person: string | null;
}

/** What follows "Nothing in <level>": " for you" or " for <first name>" when only Me or the person chip empties
 *  the view, else " → <view>" when the view has its own conditions, else null (also while unknown). */
export function useEmptyReason(listId: number, viewId: number, narrowing: Narrowing, me: MeNarrowing) {
  const { data: conditions } = useListConditions(listId, viewId);
  const { data: views } = useViews(listId);
  const viewName = views?.find((v) => v.id === viewId)?.name;
  const own = narrowing.listDraft ?? conditions?.own.filter(isComplete) ?? [];
  const viewOwn = narrowing.viewDraft ?? conditions?.view.filter(isComplete) ?? [];
  const fixed = conditions?.fixed ? [conditions.fixed] : [];
  const byPerson = me.on || me.person !== null;
  // The count leaves out the text and epic narrowings, so it can only tell Me's part alone.
  const countable = byPerson && conditions !== undefined && narrowing.text.trim() === "" && narrowing.epic === null;
  const filter = { conditions: [...fixed, ...own, ...viewOwn] };
  // The person chip isn't part of the count either, so one count without Me answers both.
  const withMe = false;
  const { data: count } = useQuery({
    queryKey: ["matchCount", listId, filter.conditions, withMe],
    queryFn: () => api.listMatchCount(listId, filter, withMe),
    enabled: countable,
  });
  const { data: people } = useViewPeople(listId, viewId, me.person ? narrowing : undefined);
  const viewFiltered = viewOwn.length > 0;

  return { suffix: suffix(), viewFiltered };

  function suffix() {
    const viewSuffix = viewFiltered && viewName ? ` → ${viewName}` : null;
    if (countable && count === undefined) return null;
    if (!countable || count === undefined || count.total === 0) return viewSuffix;
    if (me.on) return " for you";
    const name = people?.find((p) => p.accountId === me.person)?.name;
    return name ? ` for ${name.split(" ")[0]}` : viewSuffix;
  }
}
