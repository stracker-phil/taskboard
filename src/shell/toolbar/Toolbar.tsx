import { Columns3, Layers, ListFilter } from "lucide-react";
import { useCallback, useRef, useState } from "react";
import { api, type View } from "../../api";
import { narrowingKey, NO_NARROWING } from "../../state/slices/narrowing";
import { fail, useStore } from "../../state/store";
import { spaceOf, type Level } from "../levels";
import { ColumnsPopover } from "./ColumnsPopover";
import { FilterPopover } from "./FilterPopover";
import { GROUP_NAMES, groupMenu } from "./groupMenu";
import { MeChip } from "./MeChip";
import { TextFilter } from "./TextFilter";
import { useViewEdit } from "./useViewEdit";

type Open = "columns" | "filter" | null;

/** Group and Columns on the left; Filter, the Me chip and the text filter on the right. One popover at a time. */
export function Toolbar({ level, view }: { level: Level; view: View }) {
  const [open, setOpen] = useState<Open>(null);
  const columnsRef = useRef<HTMLButtonElement>(null);
  const filterRef = useRef<HTMLButtonElement>(null);
  const edit = useViewEdit();
  const key = narrowingKey(view.listId, view.id);
  const narrowing = useStore((s) => s.narrowings[key]) ?? NO_NARROWING;
  const setNarrowing = useStore((s) => s.setNarrowing);
  const setText = useCallback((text: string) => setNarrowing(key, { text }), [key, setNarrowing]);

  const toggle = (which: Exclude<Open, null>) => setOpen((current) => (current === which ? null : which));
  const close = useCallback(() => setOpen(null), []);
  const openGroupMenu = () =>
    void api
      .popupMenu(
        groupMenu(
          view.groupBy,
          view.sort,
          (groupBy) => edit(api.viewSetGroup(view.id, groupBy), "Could not change the grouping"),
          (sort) => edit(api.viewSetSort(view.id, sort), "Could not change the sort order"),
        ),
      )
      .catch((e: unknown) => fail(e, "Could not open the menu"));

  const space = spaceOf(level);
  const own = level.kind === "space" ? 0 : level.node.ownConditions;
  const listName = level.kind === "space" ? level.space.name : level.node.name;
  const columnsLabel = view.layout === "board" ? "Card fields" : "Columns";

  return (
    <div className="toolbar">
      <button type="button" className="toolbar-btn" onClick={openGroupMenu}>
        <Layers size={14} strokeWidth={2} aria-hidden="true" />
        Group: {GROUP_NAMES[view.groupBy]}
      </button>
      <span className="toolbar__anchor">
        <button
          ref={columnsRef}
          type="button"
          className={open === "columns" ? "toolbar-btn toolbar-btn--open" : "toolbar-btn"}
          aria-expanded={open === "columns"}
          onClick={() => toggle("columns")}
        >
          <Columns3 size={14} strokeWidth={2} aria-hidden="true" />
          {columnsLabel}
        </button>
        {open === "columns" && <ColumnsPopover view={view} spaceProjectId={space?.projectId ?? null} anchor={columnsRef} onClose={close} />}
      </span>
      <span className="toolbar__spacer" />
      <span className="toolbar__anchor toolbar__anchor--end">
        <button
          ref={filterRef}
          type="button"
          className={open === "filter" ? "toolbar-btn toolbar-btn--filter toolbar-btn--active" : "toolbar-btn toolbar-btn--filter"}
          aria-expanded={open === "filter"}
          onClick={() => toggle("filter")}
        >
          <ListFilter size={14} strokeWidth={2} aria-hidden="true" />
          Filter
          {own > 0 && (
            <span className="toolbar-badge" aria-label={own === 1 ? "1 condition" : `${own} conditions`}>
              {own}
            </span>
          )}
        </button>
        {open === "filter" && <FilterPopover listId={view.listId} listName={listName} anchor={filterRef} onClose={close} />}
      </span>
      <MeChip on={narrowing.me} onChange={(me) => setNarrowing(key, { me })} />
      <TextFilter key={key} value={narrowing.text} onChange={setText} />
    </div>
  );
}
