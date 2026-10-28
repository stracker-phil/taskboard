import { useQueryClient } from "@tanstack/react-query";
import { useCallback } from "react";
import type { View } from "../../api";
import { viewsKey } from "../../queries/sidebar";
import { VIEW_ROWS_KEY } from "../../queries/views";
import { fail } from "../../state/store";

/** Runs a view change and puts Rust's view into the tabs' cache; the rows refetch. */
export function useViewEdit() {
  const client = useQueryClient();
  return useCallback(
    (change: Promise<View>, failure: string) =>
      void change.then(
        (view) => {
          client.setQueryData<View[]>(viewsKey(view.listId), (views) => views?.map((v) => (v.id === view.id ? view : v)));
          void client.invalidateQueries({ queryKey: VIEW_ROWS_KEY });
        },
        (e: unknown) => fail(e, failure),
      ),
    [client],
  );
}
