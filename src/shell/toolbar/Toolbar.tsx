import { Columns3, Ellipsis, Layers, ListFilter } from "lucide-react";
import { type ReactNode, useCallback, useRef, useState } from "react";
import { type Presence, usePresence } from "../../lib/motion";
import { api, type MenuEntry, type View } from "../../api";
import { narrowingKey, NO_NARROWING } from "../../state/slices/narrowing";
import { fail, useStore } from "../../state/store";
import { spaceOf, type Level } from "../levels";
import { ColumnsPopover } from "./ColumnsPopover";
import { FilterPopover } from "./FilterPopover";
import type { Collapsible } from "./fit";
import { GROUP_NAMES, groupMenu } from "./groupMenu";
import { MeChip } from "./MeChip";
import { TextFilter } from "./TextFilter";
import { useToolbarFit } from "./useToolbarFit";
import { useViewEdit } from "./useViewEdit";

type Open = "columns" | "filter" | null;

/** Group and Columns on the left; Filter, the Me chip and the text filter on the right. One popover at a time.
 *  When the bar is too narrow, the text filter shrinks and then items move into a "…" menu. */
export function Toolbar({ level, view }: { level: Level; view: View }) {
  const [open, setOpen] = useState<Open>(null);
  const barRef = useRef<HTMLDivElement>(null);
  const columnsRef = useRef<HTMLButtonElement>(null);
  const filterRef = useRef<HTMLButtonElement>(null);
  const moreRef = useRef<HTMLButtonElement>(null);
  const { hidden, measure } = useToolbarFit(barRef);
  const edit = useViewEdit();
  const key = narrowingKey(view.listId, view.id);
  const narrowing = useStore((s) => s.narrowings[key]) ?? NO_NARROWING;
  const setNarrowing = useStore((s) => s.setNarrowing);
  const setText = useCallback((text: string) => setNarrowing(key, { text }), [key, setNarrowing]);

  const columnsShown = usePresence(open === "columns");
  const filterShown = usePresence(open === "filter");
  const toggle = (which: Exclude<Open, null>) => setOpen((current) => (current === which ? null : which));
  const close = useCallback(() => setOpen(null), []);
  const groupEntries = () =>
    groupMenu(
      view.groupBy,
      view.sort,
      (groupBy) => edit(api.viewSetGroup(view.id, groupBy), "Could not change the grouping"),
      (sort) => edit(api.viewSetSort(view.id, sort), "Could not change the sort order"),
    );
  const popup = (entries: MenuEntry[]) => void api.popupMenu(entries).catch((e: unknown) => fail(e, "Could not open the menu"));

  const space = spaceOf(level);
  const own = level.kind === "space" ? 0 : level.node.ownConditions;
  const listName = level.kind === "space" ? level.space.name : level.node.name;
  const columnsLabel = view.layout === "board" ? "Card fields" : "Columns";
  const shows = (item: Collapsible) => !hidden.includes(item);

  const moreEntries = (): MenuEntry[] =>
    hidden.map((item): MenuEntry => {
      switch (item) {
        case "columns":
          return { text: `${columnsLabel}…`, run: () => setOpen("columns") };
        case "filter":
          return { text: own > 0 ? `Filter · ${own}…` : "Filter…", run: () => setOpen("filter") };
        case "group":
          return { text: `Group: ${GROUP_NAMES[view.groupBy]}`, items: groupEntries() };
        case "me":
          return { text: "Only my issues", checked: narrowing.me, run: () => setNarrowing(key, { me: !narrowing.me }) };
      }
    });

  const columnsPopover = (anchor: typeof columnsRef) =>
    columnsShown.mounted && (
      <Fading state={columnsShown.state}>
        <ColumnsPopover view={view} spaceProjectId={space?.projectId ?? null} anchor={anchor} onClose={close} />
      </Fading>
    );
  const filterPopover = (anchor: typeof filterRef) =>
    filterShown.mounted && (
      <Fading state={filterShown.state}>
        <FilterPopover listId={view.listId} listName={listName} anchor={anchor} onClose={close} />
      </Fading>
    );

  return (
    <div ref={barRef} className="toolbar">
      {shows("group") && (
        <button ref={measure("group")} type="button" className="toolbar-btn" onClick={() => popup(groupEntries())}>
          <Layers size={14} strokeWidth={2} aria-hidden="true" />
          Group: {GROUP_NAMES[view.groupBy]}
        </button>
      )}
      {shows("columns") && (
        <span ref={measure("columns")} className="toolbar__anchor">
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
          {columnsPopover(columnsRef)}
        </span>
      )}
      <span className="toolbar__spacer" />
      {shows("filter") && (
        <span ref={measure("filter")} className="toolbar__anchor toolbar__anchor--end">
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
          {filterPopover(filterRef)}
        </span>
      )}
      {shows("me") && (
        <span ref={measure("me")} className="toolbar__item">
          <MeChip on={narrowing.me} onChange={(me) => setNarrowing(key, { me })} />
        </span>
      )}
      <TextFilter key={key} value={narrowing.text} onChange={setText} />
      {hidden.length > 0 && (
        <span className="toolbar__anchor toolbar__anchor--end">
          <button ref={moreRef} type="button" className="toolbar-btn toolbar-btn--icon" aria-label="More toolbar items" onClick={() => popup(moreEntries())}>
            <Ellipsis size={14} strokeWidth={2} aria-hidden="true" />
          </button>
          {!shows("columns") && columnsPopover(moreRef)}
          {!shows("filter") && filterPopover(moreRef)}
        </span>
      )}
    </div>
  );
}

/** Plays a popover's exit while it is still mounted; a leaving popover takes no input. */
function Fading({ state, children }: { state: Presence; children: ReactNode }) {
  return (
    <span className={`fading fading--${state}`} inert={state === "leaving"}>
      {children}
    </span>
  );
}
