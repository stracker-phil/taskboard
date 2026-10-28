import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { api, type Narrowing } from "../api";
import { NO_NARROWING } from "../state/slices/narrowing";

export { NO_NARROWING };

/** Every view's rows; `viewRowsKey` is one view's with one narrowing. */
export const VIEW_ROWS_KEY = ["view"] as const;
export const viewRowsKey = (listId: number, viewId: number, narrowing: Narrowing) => [...VIEW_ROWS_KEY, listId, viewId, narrowing] as const;
export const COLUMNS_KEY = ["columns"] as const;
export const STATUS_STYLES_KEY = ["statusStyles"] as const;


/** A view's grouped rows; the previous result stays on screen while the next one loads. */
export const useViewRows = (listId: number, viewId: number, narrowing: Narrowing = NO_NARROWING) =>
  useQuery({
    queryKey: viewRowsKey(listId, viewId, narrowing),
    queryFn: () => api.viewRows(listId, viewId, narrowing),
    placeholderData: keepPreviousData,
  });

/** Every column a view can show, for the column names. */
export const useColumnsCatalogue = () => useQuery({ queryKey: COLUMNS_KEY, queryFn: api.columnsCatalogue });

export const useStatusStyles = () => useQuery({ queryKey: STATUS_STYLES_KEY, queryFn: api.statusStylesList });

/** A level's locked and own conditions, for the Filter popover. */
export const LIST_CONDITIONS_KEY = ["listConditions"] as const;
export const listConditionsKey = (listId: number) => [...LIST_CONDITIONS_KEY, listId] as const;
export const useListConditions = (listId: number) =>
  useQuery({ queryKey: listConditionsKey(listId), queryFn: () => api.listConditions(listId) });

export const FILTER_OPTIONS_KEY = ["filterOptions"] as const;
export const useFilterOptions = () => useQuery({ queryKey: FILTER_OPTIONS_KEY, queryFn: api.filterOptions });
