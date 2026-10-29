import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { ListFilter, Lock } from "lucide-react";
import { useEffect, useId, useRef, useState, type RefObject } from "react";
import { api, type Condition, type FilterOptions, type ListConditions, type LockedCondition } from "../../api";
import { Popover } from "../../components/Popover";
import { useNavGo, useSidebar } from "../../queries/sidebar";
import { useFilterOptions, useListConditions } from "../../queries/views";
import { attempt } from "../../state/requests";
import { ConditionRow } from "./ConditionRow";
import { isComplete, newCondition } from "./conditionEditing";

const SAVE_DELAY_MS = 300;
const NEW_LIST_NAME = "New filter list";

interface Props {
  listId: number;
  viewId: number;
  /** The filter list's name, for "Changes save to …". */
  listName: string;
  anchor: RefObject<HTMLElement | null>;
  onClose: () => void;
}

/** "Show issues where": the locked conditions, then the filter list's own ones (every view of it) and the
 *  view's own ones, each saved as they change, and a live count. Built-in levels only offer the view's. */
export function FilterPopover({ listId, viewId, listName, anchor, onClose }: Props) {
  const { data: conditions } = useListConditions(listId, viewId);
  const { data: options } = useFilterOptions();
  return (
    <Popover label="Filter" anchor={anchor} onClose={onClose} className="filter-popover">
      {conditions && options ? <Editor key={`${listId}:${viewId}`} listId={listId} viewId={viewId} listName={listName} conditions={conditions} options={options} onClose={onClose} /> : null}
    </Popover>
  );
}

interface EditorProps {
  listId: number;
  viewId: number;
  listName: string;
  conditions: ListConditions;
  options: FilterOptions;
  onClose: () => void;
}

function Editor({ listId, viewId, listName, conditions, options: allOptions, onClose }: EditorProps) {
  const editsList = conditions.kind === "list";
  const options = conditions.projectFixed ? { ...allOptions, fields: allOptions.fields.filter((f) => f.id !== "project") } : allOptions;
  const [listDraft, setListDraft] = useState<Condition[]>(conditions.own);
  const [viewDraft, setViewDraft] = useState<Condition[]>(conditions.view);
  const listComplete = listDraft.filter(isComplete);
  const viewComplete = viewDraft.filter(isComplete);
  const count = useMatchCount(listId, [...listComplete, ...viewComplete]);
  useSaveWhileEditing(listDraft, conditions.own, editsList ? { kind: "list", id: listId } : null);
  useSaveWhileEditing(viewDraft, conditions.view, { kind: "view", id: viewId });

  const { data: tree } = useSidebar();
  const go = useNavGo();
  const offersNewList = conditions.kind === "everything" || conditions.kind === "space";
  const saveAsList = async () => {
    const parent = tree?.spaces.some((s) => s.listId === listId) ? listId : tree?.everything.id;
    if (parent === undefined) return;
    const id = await attempt(api.listCreate(parent, NEW_LIST_NAME, { conditions: viewComplete }), "Could not create the filter list");
    if (id === null) return;
    onClose();
    await go(id);
  };
  const narrowed = editsList || listDraft.length + viewDraft.length > 0;

  return (
    <>
      <div className="popover__header filter-popover__header">
        <ListFilter size={14} aria-hidden="true" className="filter-popover__icon" />
        <span className="popover__title">Show issues where</span>
        {count && <span className="popover__meta">{narrowed ? matchText(count.open) : openText(count.open)}</span>}
      </div>
      <div className="filter-popover__body">
        {conditions.locked.map((locked, i) => (
          <LockedRow key={i} condition={locked} />
        ))}
        {!editsList && conditions.error && <p className="filter-popover__error">{conditions.error}</p>}
        {editsList && <ConditionSection title="All views of this list" draft={listDraft} setDraft={setListDraft} options={options} error={conditions.error} />}
        <ConditionSection title="This view" draft={viewDraft} setDraft={setViewDraft} options={options} error={conditions.viewError} />
      </div>
      <div className="popover__footer">
        <span className="popover__footer-text">{editsList ? `Changes save to “${listName}” and this view.` : "Changes save to this view."}</span>
        {offersNewList && (
          <button type="button" className="btn-secondary" disabled={viewComplete.length === 0} onClick={() => void saveAsList()}>
            Save as filter list
          </button>
        )}
        <button type="button" className="btn-primary-dark" onClick={onClose}>
          Done
        </button>
      </div>
    </>
  );
}

interface SectionProps {
  title: string;
  draft: Condition[];
  setDraft: (update: (all: Condition[]) => Condition[]) => void;
  options: FilterOptions;
  error: string | null;
}

/** One layer's own conditions under a small heading, with its own "+ Add condition". */
function ConditionSection({ title, draft, setDraft, options, error }: SectionProps) {
  const headingId = useId();
  const change = (index: number, condition: Condition) => setDraft((all) => all.map((c, i) => (i === index ? condition : c)));
  const remove = (index: number) => setDraft((all) => all.filter((_, i) => i !== index));
  const add = () => setDraft((all) => [...all, newCondition(options.fields[0], options)]);
  return (
    <div role="group" aria-labelledby={headingId} className="filter-popover__section">
      <h3 id={headingId} className="popover__caps filter-popover__caps">
        {title}
      </h3>
      {error && <p className="filter-popover__error">{error}</p>}
      {draft.map((condition, i) => (
        <ConditionRow key={i} condition={condition} options={options} onChange={(c) => change(i, c)} onRemove={() => remove(i)} />
      ))}
      <button type="button" className="filter-popover__add" onClick={add}>
        + Add condition
      </button>
    </div>
  );
}

function LockedRow({ condition }: { condition: LockedCondition }) {
  return (
    <div className="locked-condition">
      <Lock size={12} strokeWidth={2.2} aria-hidden="true" />
      <span className="locked-condition__field">{condition.field}</span>
      <span>{condition.op}</span>
      {condition.value && <span className="locked-condition__value">{condition.value}</span>}
      <span className="locked-condition__source">{condition.source}</span>
    </div>
  );
}

/** Open issues, like the sidebar counts. */
const matchText = (n: number) => (n === 1 ? "1 issue matches" : `${n} issues match`);
const openText = (n: number) => (n === 1 ? "1 open issue" : `${n} open issues`);

/** The level's count with the drafts in place of its own and the view's conditions. */
function useMatchCount(listId: number, conditions: Condition[]) {
  const { data } = useQuery({
    queryKey: ["matchCount", listId, conditions],
    queryFn: () => api.listMatchCount(listId, { conditions }),
    placeholderData: keepPreviousData,
  });
  return data;
}

interface SaveTarget {
  kind: "list" | "view";
  id: number;
}

/** Saves the complete conditions to the filter list or the view shortly after each change; none saves nothing. */
function useSaveWhileEditing(draft: Condition[], saved: Condition[], target: SaveTarget | null) {
  const last = useRef(JSON.stringify(saved.filter(isComplete)));
  const kind = target?.kind;
  const id = target?.id;
  useEffect(() => {
    if (kind === undefined || id === undefined) return;
    const conditions = draft.filter(isComplete);
    const json = JSON.stringify(conditions);
    if (json === last.current) return;
    const timer = setTimeout(() => {
      last.current = json;
      const saving: Promise<unknown> = kind === "list" ? api.listSetFilter(id, { conditions }) : api.viewSetFilter(id, { conditions });
      void attempt(saving, "Could not save the filter");
    }, SAVE_DELAY_MS);
    return () => clearTimeout(timer);
  }, [kind, id, draft]);
}
