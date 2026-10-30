import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { api, type Narrowing } from "../api";
import { isComplete } from "../shell/toolbar/conditionEditing";
import { narrowingKey, NO_VIEW_NARROWING, type ViewNarrowing } from "../state/slices/narrowing";
import { useStore } from "../state/store";
import { useViews } from "./sidebar";

/** Rows asked for without any narrowing or draft. */
export const NO_NARROWING: Narrowing = { me: false, text: "", epic: null, listDraft: null, viewDraft: null };

/** What a view's rows are asked with (ADR 0038): the toolbar's narrowing, the Me chip as drafted or else as the
 *  view saved it, and the drafts' complete conditions in place of the saved ones. */
export function requestNarrowing(narrowing: ViewNarrowing, savedOnlyMine: boolean): Narrowing {
  return {
    me: narrowing.me ?? savedOnlyMine,
    text: narrowing.text,
    epic: narrowing.epic,
    listDraft: narrowing.list?.filter(isComplete) ?? null,
    viewDraft: narrowing.view?.filter(isComplete) ?? null,
  };
}

/** The narrowing a view's rows are asked with; undefined until the level's views load. */
export function useRequestNarrowing(listId: number, viewId: number): Narrowing | undefined {
  const narrowing = useStore((s) => s.narrowings[narrowingKey(listId, viewId)]) ?? NO_VIEW_NARROWING;
  const { data: views } = useViews(listId);
  const view = views?.find((v) => v.id === viewId);
  return view ? requestNarrowing(narrowing, view.onlyMine) : undefined;
}

/** Every view's rows; `viewRowsKey` is one view's with one narrowing. */
export const VIEW_ROWS_KEY = ["view"] as const;
export const viewRowsKey = (listId: number, viewId: number, narrowing: Narrowing) => [...VIEW_ROWS_KEY, listId, viewId, narrowing] as const;
export const COLUMNS_KEY = ["columns"] as const;
export const STATUS_STYLES_KEY = ["statusStyles"] as const;


/** Every typed prefix of the text filter is its own entry: unused ones go soon. */
const VIEW_GC_MS = 15_000;
const VIEW_FAILURE = "Could not load this view";

/** A view's grouped rows; the previous result stays on screen while the next one loads. */
export const useViewRows = (listId: number, viewId: number, narrowing: Narrowing = NO_NARROWING) =>
  useQuery({
    queryKey: viewRowsKey(listId, viewId, narrowing),
    queryFn: () => api.viewRows(listId, viewId, narrowing),
    placeholderData: keepPreviousData,
    gcTime: VIEW_GC_MS,
    meta: { failure: VIEW_FAILURE },
  });

/** A Board view's columns and cards; under `VIEW_ROWS_KEY`, so it refreshes with the rows. */
export const useViewBoard = (listId: number, viewId: number, narrowing: Narrowing = NO_NARROWING) =>
  useQuery({
    queryKey: [...VIEW_ROWS_KEY, "board", listId, viewId, narrowing],
    queryFn: () => api.viewBoard(listId, viewId, narrowing),
    placeholderData: keepPreviousData,
    gcTime: VIEW_GC_MS,
    meta: { failure: VIEW_FAILURE },
  });

/** Every column a view can show, for the column names. */
export const useColumnsCatalogue = (listId: number) =>
  useQuery({ queryKey: [...COLUMNS_KEY, listId], queryFn: () => api.columnsCatalogue(listId) });

export const useStatusStyles = (projectId: string) =>
  useQuery({ queryKey: [...STATUS_STYLES_KEY, projectId], queryFn: () => api.statusStylesList(projectId) });
export const SPACE_CONFIG_KEY = ["spaceConfig"] as const;
export const useSpaceConfig = (projectId: string) =>
  useQuery({ queryKey: [...SPACE_CONFIG_KEY, projectId], queryFn: () => api.spaceConfig(projectId) });

/** A level's locked and own conditions and the view's own ones, for the Filter popover. */
export const LIST_CONDITIONS_KEY = ["listConditions"] as const;
export const listConditionsKey = (listId: number, viewId: number) => [...LIST_CONDITIONS_KEY, listId, viewId] as const;
export const useListConditions = (listId: number, viewId: number) =>
  useQuery({ queryKey: listConditionsKey(listId, viewId), queryFn: () => api.listConditions(listId, viewId) });

export const FILTER_OPTIONS_KEY = ["filterOptions"] as const;
export const useFilterOptions = () => useQuery({ queryKey: FILTER_OPTIONS_KEY, queryFn: api.filterOptions });
