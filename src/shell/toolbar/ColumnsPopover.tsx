import { Search } from "lucide-react";
import { type RefObject, useState } from "react";
import { api, type Column, type IssueOpening, type View } from "../../api";
import { Popover } from "../../components/Popover";
import { SegmentedControl } from "../../components/SegmentedControl";
import { Toggle } from "../../components/Toggle";
import { useSettings } from "../../queries/settings";
import { useColumnsCatalogue } from "../../queries/views";
import { attempt } from "../../state/requests";
import { useStore } from "../../state/store";
import { DropLine } from "../Sidebar/DropLine";
import { useDragReorder } from "../Sidebar/useDragReorder";
import { useViewEdit } from "./useViewEdit";

const HINTS: Record<string, string> = { epic: "parent", release: "fix version" };
/** The popover names the due column in full; the column header stays short. */
const LABELS: Record<string, string> = { due: "Due date", labels: "Labels (column)" };
/** A Board's labels are a card field, not a column. */
const BOARD_LABELS: Record<string, string> = { ...LABELS, labels: "Labels (card field)" };

type Opening = IssueOpening | "default";
const OPENINGS: Opening[] = ["default", "preview", "full"];
const OPENING_NAMES: Record<IssueOpening, string> = { preview: "Preview", full: "Full view" };

interface Props {
  view: View;
  /** The Space whose new views "Use as Space default" sets; none outside a Space. */
  spaceProjectId: string | null;
  anchor: RefObject<HTMLElement | null>;
  onClose: () => void;
}

/** "Columns for this view": a toggle per field and drag handles for the order; every change saves to the view. */
export function ColumnsPopover({ view, spaceProjectId, anchor, onClose }: Props) {
  const { data: catalog = [] } = useColumnsCatalogue(view.listId);
  const [search, setSearch] = useState("");
  const edit = useViewEdit();
  const toast = useStore((s) => s.toast);
  const board = view.layout === "board";
  const names = board ? BOARD_LABELS : LABELS;
  const save = (columns: string[]) => edit(api.viewSetColumns(view.id, columns), "Could not change the columns");

  // The order the list opened with stays while it is open: a toggle changes only its switch, never which field
  // is under the cursor (docs/ux.md → Stable controls). A drag sets a new order.
  const [order, setOrder] = useState<string[] | null>(null);
  if (order === null && catalog.length > 0) setOrder(inViewOrder(catalog, view.columns).map((c) => c.id));
  const listed = (columns: Column[]) => (order ? inOrder(columns, order) : inViewOrder(columns, view.columns));
  const standard = listed(catalog.filter((c) => !c.custom));
  const custom = listed(catalog.filter((c) => c.custom));
  const listedIds = [...standard, ...custom].map((c) => c.id);
  /** Each section keeps its own order; the view shows the Jira fields first. */
  const saveOrder = (nextStandard: Column[], nextCustom: Column[]) => {
    const ids = [...nextStandard, ...nextCustom].map((c) => c.id);
    setOrder(ids);
    save(ids.filter((id) => view.columns.includes(id)));
  };
  const toggle = (id: string) =>
    save(view.columns.includes(id) ? view.columns.filter((c) => c !== id) : listedIds.filter((c) => c === id || view.columns.includes(c)));

  // While searching, only matching fields show and their order can't change: a drag among some of them
  // would lose the place of the others.
  const needle = search.trim().toLowerCase();
  const visible = (columns: Column[]) => (needle ? columns.filter((c) => matches(c, needle, names)) : columns);
  const shownStandard = visible(standard);
  const shownCustom = visible(custom);

  const saveAsSpaceDefault = async () => {
    if (!spaceProjectId) return;
    const done = await attempt(api.spaceDefaultColumns(spaceProjectId, view.columns), "Could not save the Space's default columns");
    if (done !== null) toast("success", "New views in this Space start with these columns");
  };

  return (
    <Popover label="Columns for this view" anchor={anchor} onClose={onClose} className="columns-popover">
      <div className="popover__header">
        <span className="popover__title">Columns</span>
        <span className="popover__meta">View: {view.name}</span>
      </div>
      {!board && <OpenWith view={view} onChange={(openWith) => edit(api.viewSetOpenWith(view.id, openWith), "Could not change how issues open")} />}
      <button
        type="button"
        className="column-row columns-popover__wrap"
        aria-pressed={view.inlineLabels}
        onClick={() => edit(api.viewSetInlineLabels(view.id, !view.inlineLabels), "Could not change how labels show")}
      >
        <span className="column-row__label">Labels (inline)</span>
        <span className="column-row__hint">{view.columns.includes("labels") ? `off while the ${board ? "card field" : "column"} shows` : "after the title"}</span>
        <Toggle on={view.inlineLabels} />
      </button>
      {view.layout === "list" && (
        <button
          type="button"
          className="column-row columns-popover__wrap"
          aria-pressed={view.wrapTitles}
          onClick={() => edit(api.viewSetWrap(view.id, !view.wrapTitles), "Could not change how titles show")}
        >
          <span className="column-row__label">Wrap long titles</span>
          <span className="column-row__hint">{view.wrapTitles ? "on" : "truncate"}</span>
          <Toggle on={view.wrapTitles} />
        </button>
      )}
      <label className="columns-popover__search">
        <Search size={13} strokeWidth={2} aria-hidden="true" />
        <input aria-label="Search fields" placeholder="Search fields" value={search} onChange={(e) => setSearch(e.target.value)} />
      </label>
      <div className="columns-popover__body">
        {shownStandard.length > 0 && (
          <>
            <div className="popover__caps">Jira fields</div>
            <Section columns={shownStandard} shown={view.columns} onToggle={toggle} onReorder={(next) => saveOrder(next, custom)} reorderable={!needle} names={names} />
          </>
        )}
        {shownCustom.length > 0 && (
          <>
            <div className="popover__caps popover__caps--spaced">Custom fields · this project</div>
            <Section columns={shownCustom} shown={view.columns} onToggle={toggle} onReorder={(next) => saveOrder(standard, next)} reorderable={!needle} names={names} mono />
          </>
        )}
        {shownStandard.length === 0 && shownCustom.length === 0 && <p className="columns-popover__empty">No field matches “{search.trim()}”.</p>}
      </div>
      <div className="popover__footer">
        <span className="popover__footer-text">Saved for this view only</span>
        {spaceProjectId && (
          <button type="button" className="popover__small-btn" onClick={() => void saveAsSpaceDefault()}>
            Use as Space default
          </button>
        )}
      </div>
    </Popover>
  );
}

/** By the name shown, the Jira id or its hint ("cf_10071", "fix version"). */
function matches(column: Column, needle: string, names: Record<string, string>): boolean {
  const hint = column.custom ? column.id.replace("customfield_", "cf_") : (HINTS[column.id] ?? "");
  return [names[column.id] ?? column.name, column.id, hint].some((text) => text.toLowerCase().includes(needle));
}

/** `columns` in `order`; any it doesn't know follow, in the catalog's order. */
function inOrder(columns: Column[], order: string[]): Column[] {
  const rank = (column: Column) => {
    const at = order.indexOf(column.id);
    return at === -1 ? order.length : at;
  };
  return [...columns].sort((a, b) => rank(a) - rank(b));
}

/** The view's columns in its order, then the others in the catalog's. */
function inViewOrder(columns: Column[], shown: string[]): Column[] {
  const on = shown.map((id) => columns.find((c) => c.id === id)).filter((c): c is Column => !!c);
  return [...on, ...columns.filter((c) => !shown.includes(c.id))];
}

interface SectionProps {
  columns: Column[];
  shown: string[];
  onToggle: (id: string) => void;
  onReorder: (columns: Column[]) => void;
  reorderable: boolean;
  /** Names that differ from the field's own. */
  names: Record<string, string>;
  mono?: boolean;
}

function Section({ columns, shown, onToggle, onReorder, reorderable, names, mono = false }: SectionProps) {
  const { containerRef, item, handle, dropLineY } = useDragReorder<HTMLDivElement>((from, to) => {
    const next = [...columns];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    onReorder(next);
  });
  return (
    <div className="columns-popover__section" ref={containerRef}>
      {columns.map((column, index) => {
        const on = shown.includes(column.id);
        const hint = column.custom ? column.id.replace("customfield_", "cf_") : HINTS[column.id];
        return (
          <button
            key={column.id}
            type="button"
            className="column-row"
            aria-pressed={on}
            onClick={() => onToggle(column.id)}
            {...item(index)}
            {...(reorderable ? handle(index) : {})}
          >
            <span className={reorderable ? "column-row__handle" : "column-row__handle column-row__handle--off"} aria-hidden="true">
              ⋮⋮
            </span>
            <span className="column-row__label">{names[column.id] ?? column.name}</span>
            {hint && <span className={mono ? "column-row__hint mono" : "column-row__hint"}>{hint}</span>}
            <Toggle on={on} />
          </button>
        );
      })}
      <DropLine y={dropLineY} />
    </div>
  );
}

/** What a click on an issue opens in this view; Default follows Settings → General. */
function OpenWith({ view, onChange }: { view: View; onChange: (openWith: IssueOpening | null) => void }) {
  const { data: settings } = useSettings();
  const appDefault = OPENING_NAMES[settings?.behaviour.openIssuesIn ?? "preview"];
  return (
    <div className="columns-popover__open">
      <span className="column-row__label">Open issues in</span>
      <SegmentedControl<Opening>
        label="Open issues in"
        options={OPENINGS}
        labels={{ default: `Default (${appDefault})`, ...OPENING_NAMES }}
        value={view.openWith ?? "default"}
        onChange={(choice) => onChange(choice === "default" ? null : choice)}
      />
    </div>
  );
}
