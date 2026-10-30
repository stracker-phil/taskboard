import { Columns3, Ellipsis, Layers, ListFilter } from "lucide-react";
import { type ReactNode, type RefObject, useCallback, useEffect, useRef, useState } from "react";
import { usePageAction } from "../../commands/pageActions";
import { type Presence, usePresence } from "../../lib/motion";
import { api, type View } from "../../api";
import { openMenu, type MenuEntry } from "../../components/menu";
import { narrowingKey, NO_VIEW_NARROWING } from "../../state/slices/narrowing";
import { useStore } from "../../state/store";
import { spaceOf, type Level } from "../levels";
import { ColumnsPopover } from "./ColumnsPopover";
import { EpicNarrowingChip } from "./EpicChip";
import { FilterPopover } from "./FilterPopover";
import type { Collapsible } from "./fit";
import { GROUP_NAMES, groupMenu } from "./groupMenu";
import { MODE_NAMES, SubtasksButton, subtasksMenu } from "./SubtasksButton";
import { MeChip } from "./MeChip";
import { TextFilter } from "./TextFilter";
import { SaveViewButton } from "./SaveViewButton";
import { useToolbarFit } from "./useToolbarFit";
import { useViewDraft } from "./useViewDraft";
import { useViewEdit } from "./useViewEdit";

type Open = "columns" | "filter" | null;

/** One popover at a time; when the bar is too narrow, the text filter shrinks and then items move into a "…" menu.
 *  The Me chip and the Filter popover change the view as a draft until Save View (ADR 0038). */
export function Toolbar({ level, view }: { level: Level; view: View }) {
  const [open, setOpen] = useState<Open>(null);
  const barRef = useRef<HTMLDivElement>(null);
  const columnsRef = useRef<HTMLButtonElement>(null);
  const filterRef = useRef<HTMLButtonElement>(null);
  const moreRef = useRef<HTMLButtonElement>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  // The Board always hides subtasks, so its bar has no Subtasks button.
  const hasSubtasks = view.layout !== "board";
  const draft = useViewDraft(level, view);
  const { hidden, measure } = useToolbarFit(barRef, hasSubtasks ? [] : ["subtasks"], draft.dirty);
  const edit = useViewEdit();
  const key = narrowingKey(view.listId, view.id);
  const narrowing = useStore((s) => s.narrowings[key]) ?? NO_VIEW_NARROWING;
  const setNarrowing = useStore((s) => s.setNarrowing);
  const setText = useCallback((text: string) => setNarrowing(key, { text }), [key, setNarrowing]);

  // Edit filter in an empty list opens the popover; the count a mount starts with is not a request.
  const filterRequests = useStore((s) => s.filterRequests);
  const handledRequests = useRef(filterRequests);
  useEffect(() => {
    if (filterRequests === handledRequests.current) return;
    handledRequests.current = filterRequests;
    setOpen("filter");
  }, [filterRequests]);

  const columnsShown = usePresence(open === "columns");
  const filterShown = usePresence(open === "filter");
  const toggle = (which: Exclude<Open, null>) => setOpen((current) => (current === which ? null : which));
  const close = () => setOpen(null);
  const groupEntries = () =>
    groupMenu(
      view.groupBy,
      view.sort,
      (groupBy) => edit(api.viewSetGroup(view.id, groupBy), "Could not change the grouping"),
      (sort) => edit(api.viewSetSort(view.id, sort), "Could not change the sort order"),
    );
  const groupRef = useRef<HTMLButtonElement | null>(null);
  const measureGroup = measure("group");
  const groupButton = useCallback(
    (button: HTMLButtonElement | null) => {
      groupRef.current = button;
      return measureGroup(button);
    },
    [measureGroup],
  );
  // Under the "…" button while the Group button is folded into it.
  const openGroupMenu = () => {
    const anchor = groupRef.current ?? moreRef.current;
    if (anchor) openMenu(groupEntries(), anchor, "Group");
  };

  const subtaskEntries = () => subtasksMenu(view.subtasks, (mode) => edit(api.viewSetSubtasks(view.id, mode), "Could not change how subtasks show"));

  const space = spaceOf(level);
  // Both layers count, as drafted or saved: the filter list's own conditions and the view's.
  const own = draft.conditions;
  const toggleMe = () => setNarrowing(key, { me: !draft.me });
  const save = () => void draft.save();
  const columnsLabel = view.layout === "board" ? "Card fields" : "Columns";
  const shows = (item: Collapsible) => !hidden.includes(item) && (item !== "subtasks" || hasSubtasks);

  usePageAction("toolbar.me", toggleMe);
  usePageAction("view.save", draft.dirty ? save : null);
  usePageAction("view.revert", draft.dirty ? draft.revert : null);
  usePageAction("toolbar.search", () => {
    searchRef.current?.focus();
    searchRef.current?.select();
  });
  usePageAction("toolbar.filter", () => setOpen("filter"));
  usePageAction("toolbar.columns", () => setOpen("columns"));
  usePageAction("toolbar.group", openGroupMenu);

  const moreEntries = (): MenuEntry[] => [
    ...hidden.map((item): MenuEntry => {
      switch (item) {
        case "columns":
          return { text: `${columnsLabel}…`, run: () => setOpen("columns") };
        case "subtasks":
          return { text: "Subtasks", hint: MODE_NAMES[view.subtasks], items: subtaskEntries() };
        case "filter":
          return { text: own > 0 ? `Filter · ${own}…` : "Filter…", run: () => setOpen("filter") };
        case "group":
          return { text: `Group: ${GROUP_NAMES[view.groupBy]}`, items: groupEntries() };
        case "me":
          return { text: "Only my issues", checked: draft.me, toggle: true, run: toggleMe };
      }
    }),
    ...(draft.dirty ? revertEntries(hidden.length > 0, draft.revert) : []),
  ];

  const columnsPopover = (anchor: RefObject<HTMLButtonElement | null>) =>
    columnsShown.mounted && (
      <Fading state={columnsShown.state}>
        <ColumnsPopover view={view} spaceProjectId={space?.projectId ?? null} anchor={anchor} onClose={close} />
      </Fading>
    );
  const filterPopover = (anchor: RefObject<HTMLButtonElement | null>) =>
    filterShown.mounted && (
      <Fading state={filterShown.state}>
        <FilterPopover listId={view.listId} viewId={view.id} anchor={anchor} dirty={draft.dirty} onSave={save} onClose={close} />
      </Fading>
    );

  return (
    <div ref={barRef} className="toolbar">
      {shows("group") && (
        <button ref={groupButton} type="button" className="toolbar-btn" aria-haspopup="menu" onClick={openGroupMenu}>
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
      {shows("subtasks") && <SubtasksButton ref={measure("subtasks")} mode={view.subtasks} entries={subtaskEntries} />}
      <span className="toolbar__spacer" />
      {draft.dirty && <SaveViewButton onSave={save} />}
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
          <MeChip on={draft.me} onChange={(me) => setNarrowing(key, { me })} />
        </span>
      )}
      {narrowing.epic && <EpicNarrowingChip epicId={narrowing.epic} onClear={() => setNarrowing(key, { epic: null })} />}
      <TextFilter key={key} value={narrowing.text} onChange={setText} inputRef={searchRef} />
      {(hidden.length > 0 || draft.dirty) && (
        <span className="toolbar__anchor toolbar__anchor--end">
          <button ref={moreRef} type="button" className="toolbar-btn toolbar-btn--icon" aria-label="More toolbar items" aria-haspopup="menu" onClick={(e) => openMenu(moreEntries(), e.currentTarget, "More toolbar items")}>
            <Ellipsis size={14} strokeWidth={2} aria-hidden="true" />
          </button>
          {!shows("columns") && columnsPopover(moreRef)}
          {!shows("filter") && filterPopover(moreRef)}
        </span>
      )}
    </div>
  );
}

/** Revert View, after a separator when other items come first. */
function revertEntries(afterItems: boolean, revert: () => void): MenuEntry[] {
  const entry: MenuEntry = { text: "Revert View", run: revert };
  return afterItems ? ["separator", entry] : [entry];
}

/** Plays a popover's exit while it is still mounted; a leaving popover takes no input. */
function Fading({ state, children }: { state: Presence; children: ReactNode }) {
  return (
    <span className={`fading fading--${state}`} inert={state === "leaving"}>
      {children}
    </span>
  );
}
