import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { ListFilter, Lock } from "lucide-react";
import { useEffect, useRef, useState, type RefObject } from "react";
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
  /** The filter list's name, for "Changes save to …". */
  listName: string;
  anchor: RefObject<HTMLElement | null>;
  onClose: () => void;
}

/** "Show issues where": the locked conditions, a filter list's own ones (saved as they change) and a live count.
 *  On a Space or Everything the conditions are a draft for a new filter list. */
export function FilterPopover({ listId, listName, anchor, onClose }: Props) {
  const { data: conditions } = useListConditions(listId);
  const { data: options } = useFilterOptions();
  return (
    <Popover label="Filter" anchor={anchor} onClose={onClose} className="filter-popover">
      {conditions && options ? <Editor key={listId} listId={listId} listName={listName} conditions={conditions} options={options} onClose={onClose} /> : null}
    </Popover>
  );
}

function Editor({ listId, listName, conditions, options: allOptions, onClose }: { listId: number; listName: string; conditions: ListConditions; options: FilterOptions; onClose: () => void }) {
  const editsList = conditions.kind === "list";
  const options = conditions.projectFixed ? { ...allOptions, fields: allOptions.fields.filter((f) => f.id !== "project") } : allOptions;
  const builtIn = conditions.kind === "recent" || conditions.kind === "pinned";
  const [draft, setDraft] = useState<Condition[]>(conditions.own);
  const complete = draft.filter(isComplete);
  const count = useMatchCount(listId, complete);
  useSaveWhileEditing(editsList, listId, draft, conditions.own);

  const { data: tree } = useSidebar();
  const go = useNavGo();
  const saveAsList = async () => {
    const parent = tree?.spaces.some((s) => s.listId === listId) ? listId : tree?.everything.id;
    if (parent === undefined) return;
    const id = await attempt(api.listCreate(parent, NEW_LIST_NAME, { conditions: complete }), "Could not create the filter list");
    if (id === null) return;
    onClose();
    await go(id);
  };

  const change = (index: number, condition: Condition) => setDraft((all) => all.map((c, i) => (i === index ? condition : c)));
  const remove = (index: number) => setDraft((all) => all.filter((_, i) => i !== index));
  const add = () => setDraft((all) => [...all, newCondition(options.fields[0], options)]);

  return (
    <>
      <div className="popover__header filter-popover__header">
        <ListFilter size={14} aria-hidden="true" className="filter-popover__icon" />
        <span className="popover__title">Show issues where</span>
        {count && <span className="popover__meta">{editsList || draft.length > 0 ? matchText(count.open) : openText(count.open)}</span>}
      </div>
      <div className="filter-popover__body">
        {conditions.locked.map((locked, i) => (
          <LockedRow key={i} condition={locked} />
        ))}
        {conditions.error && <p className="filter-popover__error">{conditions.error}</p>}
        {draft.map((condition, i) => (
          <ConditionRow key={i} condition={condition} options={options} onChange={(c) => change(i, c)} onRemove={() => remove(i)} />
        ))}
        {!builtIn && (
          <button type="button" className="filter-popover__add" onClick={add}>
            + Add condition
          </button>
        )}
      </div>
      <div className="popover__footer">
        <span className="popover__footer-text">{footerText(conditions, listName)}</span>
        {!editsList && !builtIn && (
          <button type="button" className="btn-secondary" disabled={complete.length === 0} onClick={() => void saveAsList()}>
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

function footerText(conditions: ListConditions, listName: string): string {
  if (conditions.kind === "list") return `Changes save to “${listName}”.`;
  if (conditions.kind === "recent" || conditions.kind === "pinned") return "This list is built in.";
  return "Adding a condition here creates a new filter list.";
}

/** The level's count with the draft in place of its own conditions. */
function useMatchCount(listId: number, conditions: Condition[]) {
  const { data } = useQuery({
    queryKey: ["matchCount", listId, conditions],
    queryFn: () => api.listMatchCount(listId, { conditions }),
    placeholderData: keepPreviousData,
  });
  return data;
}

/** A filter list saves its complete conditions shortly after each change. */
function useSaveWhileEditing(enabled: boolean, listId: number, draft: Condition[], saved: Condition[]) {
  const last = useRef(JSON.stringify(saved.filter(isComplete)));
  useEffect(() => {
    if (!enabled) return;
    const conditions = draft.filter(isComplete);
    const json = JSON.stringify(conditions);
    if (json === last.current) return;
    const timer = setTimeout(() => {
      last.current = json;
      void attempt(api.listSetFilter(listId, { conditions }), "Could not save the filter");
    }, SAVE_DELAY_MS);
    return () => clearTimeout(timer);
  }, [enabled, listId, draft]);
}
