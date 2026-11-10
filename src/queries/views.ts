import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { api, type Narrowing, type View } from "../api";
import { isComplete } from "../shell/toolbar/conditionEditing";
import { narrowingKey, NO_VIEW_NARROWING, type ViewNarrowing } from "../state/slices/narrowing";
import { useStore } from "../state/store";
import { useViews } from "./sidebar";

/** Rows asked for without any narrowing or draft. */
export const NO_NARROWING: Narrowing = { me: false, text: "", epic: null, person: null, listDraft: null, viewDraft: null, meOn: null };

/** What a view's rows are asked with (ADR 0038): the toolbar's narrowing, the Me chip as drafted or else as the
 *  view saved it, and the drafts' complete conditions in place of the saved ones. */
/** The drafts over what the view saved. */
export function requestNarrowing(narrowing: ViewNarrowing, saved: Pick<View, "onlyMine" | "person" | "meRules">): Narrowing {
  return {
    me: narrowing.me ?? saved.onlyMine,
    text: narrowing.text,
    epic: narrowing.epic,
    person: narrowing.person === undefined ? saved.person : narrowing.person,
    listDraft: narrowing.list?.filter(isComplete) ?? null,
    viewDraft: narrowing.view?.filter(isComplete) ?? null,
    meOn: narrowing.meRules === undefined ? saved.meRules : narrowing.meRules,
  };
}

/** The narrowing a view's rows are asked with; undefined until the level's views load. */
export function useRequestNarrowing(listId: number, viewId: number): Narrowing | undefined {
  const narrowing = useStore((s) => s.narrowings[narrowingKey(listId, viewId)]) ?? NO_VIEW_NARROWING;
  const { data: views } = useViews(listId);
  const view = views?.find((v) => v.id === viewId);
  return view ? requestNarrowing(narrowing, view) : undefined;
}

/** Every view's rows; `viewRowsKey` is one view's with one narrowing. */
export const VIEW_ROWS_KEY = ["view"] as const;
export const viewRowsKey = (listId: number, viewId: number, narrowing: Narrowing) => [...VIEW_ROWS_KEY, listId, viewId, narrowing] as const;
export const COLUMNS_KEY = ["columns"] as const;
export const STATUS_STYLES_KEY = ["statusStyles"] as const;

/** Every typed prefix of the text filter is its own entry: unused ones go soon. */
const VIEW_GC_MS = 15_000;
const VIEW_FAILURE = "Could not load this view";

const viewRowsQuery = (listId: number, viewId: number, narrowing: Narrowing) => ({
  queryKey: viewRowsKey(listId, viewId, narrowing),
  queryFn: () => api.viewRows(listId, viewId, narrowing),
  gcTime: VIEW_GC_MS,
  meta: { failure: VIEW_FAILURE },
});

/** Under `VIEW_ROWS_KEY`, so it refreshes with the rows. */
const viewBoardQuery = (listId: number, viewId: number, narrowing: Narrowing) => ({
  queryKey: [...VIEW_ROWS_KEY, "board", listId, viewId, narrowing],
  queryFn: () => api.viewBoard(listId, viewId, narrowing),
  gcTime: VIEW_GC_MS,
  meta: { failure: VIEW_FAILURE },
});

/** What a view's layout loads: the Board its columns and cards, the List and the Table their rows. */
export const viewDataQuery = (view: View, narrowing: Narrowing): { queryKey: readonly unknown[]; queryFn: () => Promise<unknown>; gcTime: number; meta: { failure: string } } =>
  view.layout === "board" ? viewBoardQuery(view.listId, view.id, narrowing) : viewRowsQuery(view.listId, view.id, narrowing);

/** A view's grouped rows; the previous result stays on screen while the next one loads. */
export const useViewRows = (listId: number, viewId: number, narrowing: Narrowing = NO_NARROWING) =>
  useQuery({ ...viewRowsQuery(listId, viewId, narrowing), placeholderData: keepPreviousData });

/** A Board view's columns and cards. */
export const useViewBoard = (listId: number, viewId: number, narrowing: Narrowing = NO_NARROWING) =>
  useQuery({ ...viewBoardQuery(listId, viewId, narrowing), placeholderData: keepPreviousData });

/** Every column a view can show, for the column names. */
export const useColumnsCatalogue = (listId: number) =>
  useQuery({ queryKey: [...COLUMNS_KEY, listId], queryFn: () => api.columnsCatalogue(listId) });

export const useStatusStyles = (projectId: string) =>
  useQuery({ queryKey: [...STATUS_STYLES_KEY, projectId], queryFn: () => api.statusStylesList(projectId) });

/** Under `STATUS_STYLES_KEY`, so the events that refresh a Space's statuses refresh these too. */
export const APP_STATUS_STYLES_KEY = [...STATUS_STYLES_KEY, "app"] as const;
export const useAppStatusStyles = () => useQuery({ queryKey: APP_STATUS_STYLES_KEY, queryFn: () => api.statusStylesAll() });
export const SPACE_CONFIG_KEY = ["spaceConfig"] as const;
export const useSpaceConfig = (projectId: string) =>
  useQuery({ queryKey: [...SPACE_CONFIG_KEY, projectId], queryFn: () => api.spaceConfig(projectId) });

/** Everyone with activity in a Space's issues, and who of them is switched off. */
export const TEAM_KEY = ["team"] as const;
export const TEAM_MATRIX_KEY = [...TEAM_KEY, "matrix"] as const;
export const useTeamMatrix = () => useQuery({ queryKey: TEAM_MATRIX_KEY, queryFn: () => api.teamMatrix() });
/** Whom the person chip offers for the view as it shows: me, then the assignees its filter leaves. Under
 *  `VIEW_ROWS_KEY`, so it refreshes with the rows. */
export const useViewPeople = (listId: number, viewId: number, narrowing: Narrowing | undefined) =>
  useQuery({
    queryKey: [...VIEW_ROWS_KEY, "people", listId, viewId, narrowing],
    queryFn: () => api.viewPeople(listId, viewId, narrowing as Narrowing),
    enabled: narrowing !== undefined,
    gcTime: VIEW_GC_MS,
  });
export const useMeUserFields = () => useQuery({ queryKey: [...TEAM_KEY, "meFields"], queryFn: api.meUserFields });

/** A level's locked and own conditions and the view's own ones, for the Filter popover. */
export const LIST_CONDITIONS_KEY = ["listConditions"] as const;
export const listConditionsKey = (listId: number, viewId: number) => [...LIST_CONDITIONS_KEY, listId, viewId] as const;
export const useListConditions = (listId: number, viewId: number) =>
  useQuery({ queryKey: listConditionsKey(listId, viewId), queryFn: () => api.listConditions(listId, viewId) });

export const FILTER_OPTIONS_KEY = ["filterOptions"] as const;
export const useFilterOptions = (listId: number) =>
  useQuery({ queryKey: [...FILTER_OPTIONS_KEY, listId], queryFn: () => api.filterOptions(listId) });
