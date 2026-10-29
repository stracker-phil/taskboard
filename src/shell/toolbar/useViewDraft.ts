import { useQueryClient } from "@tanstack/react-query";
import { useCallback } from "react";
import { api, type Condition, type View } from "../../api";
import { SIDEBAR_KEY, viewsKey } from "../../queries/sidebar";
import { listConditionsKey, useListConditions } from "../../queries/views";
import { fail } from "../../state/requests";
import { narrowingKey, NO_VIEW_NARROWING } from "../../state/slices/narrowing";
import { useStore } from "../../state/store";
import type { Level } from "../levels";
import { isComplete } from "./conditionEditing";

/** Conditions compare by what they filter, whatever their key order. */
const sameConditions = (a: Condition[], b: Condition[]) =>
  JSON.stringify(a.map((c) => [c.field, c.op, c.value ?? null])) === JSON.stringify(b.map((c) => [c.field, c.op, c.value ?? null]));

export interface ViewDraft {
  /** The Me chip as it shows: the draft, else what the view saved. */
  me: boolean;
  /** The Filter badge: the drafted or saved conditions of the filter list and of the view. */
  conditions: number;
  /** Something differs from what the view and its list saved. */
  dirty: boolean;
  /** Writes the drafts to the list and the view, then drops them. */
  save: () => Promise<void>;
  /** Drops the drafts: back to what is saved. */
  revert: () => void;
}

/** The current view's unsaved Me chip and filter conditions (ADR 0038). */
export function useViewDraft(level: Level, view: View): ViewDraft {
  const client = useQueryClient();
  const key = narrowingKey(view.listId, view.id);
  const narrowing = useStore((s) => s.narrowings[key]) ?? NO_VIEW_NARROWING;
  const clearDrafts = useStore((s) => s.clearDrafts);
  const { data: saved } = useListConditions(view.listId, view.id);

  const me = narrowing.me ?? view.onlyMine;
  const listDraft = narrowing.list?.filter(isComplete) ?? null;
  const viewDraft = narrowing.view?.filter(isComplete) ?? null;
  const meDirty = me !== view.onlyMine;
  const listDirty = listDraft !== null && saved !== undefined && !sameConditions(listDraft, saved.own);
  const viewDirty = viewDraft !== null && saved !== undefined && !sameConditions(viewDraft, saved.view);
  const listSaved = level.kind === "space" ? 0 : level.node.ownConditions;

  const save = useCallback(async () => {
    let latest: View | null = null;
    try {
      if (listDirty && listDraft) await api.listSetFilter(view.listId, { conditions: listDraft });
      if (viewDirty && viewDraft) latest = await api.viewSetFilter(view.id, { conditions: viewDraft });
      if (meDirty) latest = await api.viewSetOnlyMine(view.id, me);
    } catch (e) {
      // The drafts stay, so nothing is lost; what did save shows as saved.
      fail(e, "Could not save the view");
      return;
    }
    const updated: View | null = latest;
    if (updated) client.setQueryData<View[]>(viewsKey(view.listId), (views) => views?.map((v) => (v.id === updated.id ? updated : v)));
    // The saved state is in place before the drafts go, so the rows and the badge don't flicker back.
    await Promise.all([client.invalidateQueries({ queryKey: listConditionsKey(view.listId, view.id) }), client.invalidateQueries({ queryKey: SIDEBAR_KEY })]);
    clearDrafts(key);
  }, [client, clearDrafts, key, view.listId, view.id, listDirty, listDraft, viewDirty, viewDraft, meDirty, me]);

  return {
    me,
    conditions: (listDraft?.length ?? listSaved) + (viewDraft?.length ?? view.ownConditions),
    dirty: meDirty || listDirty || viewDirty,
    save,
    revert: useCallback(() => clearDrafts(key), [clearDrafts, key]),
  };
}
