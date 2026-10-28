import { useQueryClient } from "@tanstack/react-query";
import { api, type Sort, type View } from "../../api";
import { VIEWS_KEY, viewsKey } from "../../queries/sidebar";
import { VIEW_ROWS_KEY } from "../../queries/views";
import { fail } from "../../state/requests";

/** Saves a view's sort and column widths; shown at once, Rust's answer replaces it. */
export function useViewSettings(view: View) {
  const client = useQueryClient();

  const save = (change: Partial<View>, call: Promise<View>, failure: string) => {
    client.setQueryData<View[]>(viewsKey(view.listId), (views) => views?.map((v) => (v.id === view.id ? { ...v, ...change } : v)));
    return call
      .catch((e: unknown) => void fail(e, failure))
      .finally(() => {
        void client.invalidateQueries({ queryKey: VIEWS_KEY });
        if (change.sort) void client.invalidateQueries({ queryKey: VIEW_ROWS_KEY });
      });
  };

  return {
    setSort: (sort: Sort) => save({ sort }, api.viewSetSort(view.id, sort), "Could not sort the view"),
    setWidth: (id: string, width: number) => {
      const widths = { ...view.widths, [id]: width };
      return save({ widths }, api.viewSetWidths(view.id, widths), "Could not save the column width");
    },
  };
}
