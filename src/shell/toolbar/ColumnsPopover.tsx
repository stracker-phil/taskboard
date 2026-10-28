import type { RefObject } from "react";
import { api, type Column, type View } from "../../api";
import { Popover } from "../../components/Popover";
import { Toggle } from "../../components/Toggle";
import { useColumnsCatalogue } from "../../queries/views";
import { attempt } from "../../state/requests";
import { useStore } from "../../state/store";
import { DropLine } from "../Sidebar/DropLine";
import { useDragReorder } from "../Sidebar/useDragReorder";
import { useViewEdit } from "./useViewEdit";

const HINTS: Record<string, string> = { epic: "parent", release: "fix version" };
/** The popover names the due column in full; the column header stays short. */
const LABELS: Record<string, string> = { due: "Due date" };

interface Props {
  view: View;
  /** The Space whose new views "Use as Space default" sets; none outside a Space. */
  spaceProjectId: string | null;
  anchor: RefObject<HTMLElement | null>;
  onClose: () => void;
}

/** "Columns for this view": a toggle per field and drag handles for the order; every change saves to the view. */
export function ColumnsPopover({ view, spaceProjectId, anchor, onClose }: Props) {
  const { data: catalogue = [] } = useColumnsCatalogue();
  const edit = useViewEdit();
  const toast = useStore((s) => s.toast);
  const save = (columns: string[]) => edit(api.viewSetColumns(view.id, columns), "Could not change the columns");

  const standard = inViewOrder(catalogue.filter((c) => !c.custom), view.columns);
  const custom = inViewOrder(catalogue.filter((c) => c.custom), view.columns);
  const listedIds = [...standard, ...custom].map((c) => c.id);
  /** Each section keeps its own order; the view shows the Jira fields first. */
  const saveOrder = (nextStandard: Column[], nextCustom: Column[]) =>
    save([...nextStandard, ...nextCustom].map((c) => c.id).filter((id) => view.columns.includes(id)));
  const toggle = (id: string) =>
    save(view.columns.includes(id) ? view.columns.filter((c) => c !== id) : listedIds.filter((c) => c === id || view.columns.includes(c)));

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
      <div className="columns-popover__body">
        <div className="popover__caps">Jira fields</div>
        <Section columns={standard} shown={view.columns} onToggle={toggle} onReorder={(next) => saveOrder(next, custom)} />
        {custom.length > 0 && (
          <>
            <div className="popover__caps popover__caps--spaced">Custom fields · this project</div>
            <Section columns={custom} shown={view.columns} onToggle={toggle} onReorder={(next) => saveOrder(standard, next)} mono />
          </>
        )}
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

/** The view's columns in its order, then the others in the catalogue's. */
function inViewOrder(columns: Column[], shown: string[]): Column[] {
  const on = shown.map((id) => columns.find((c) => c.id === id)).filter((c): c is Column => !!c);
  return [...on, ...columns.filter((c) => !shown.includes(c.id))];
}

interface SectionProps {
  columns: Column[];
  shown: string[];
  onToggle: (id: string) => void;
  onReorder: (columns: Column[]) => void;
  mono?: boolean;
}

function Section({ columns, shown, onToggle, onReorder, mono = false }: SectionProps) {
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
          <button key={column.id} type="button" className="column-row" aria-pressed={on} onClick={() => onToggle(column.id)} {...item(index)} {...handle(index)}>
            <span className="column-row__handle" aria-hidden="true">
              ⋮⋮
            </span>
            <span className="column-row__label">{LABELS[column.id] ?? column.name}</span>
            {hint && <span className={mono ? "column-row__hint mono" : "column-row__hint"}>{hint}</span>}
            <Toggle on={on} />
          </button>
        );
      })}
      <DropLine y={dropLineY} />
    </div>
  );
}
