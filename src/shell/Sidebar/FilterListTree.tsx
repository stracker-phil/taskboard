import type { HTMLAttributes, MouseEvent } from "react";
import { Funnel } from "lucide-react";
import { api, type ListNode, type SpaceNode } from "../../api";
import { InlineRename } from "../../components/InlineRename";
import { attempt } from "../../state/requests";
import { useStore } from "../../state/store";
import { DropLine } from "./DropLine";
import { ListCount } from "./ListCount";
import { useDragReorder } from "./useDragReorder";

const NEW_LIST_NAME = "New filter list";

interface Props {
  space: SpaceNode;
  activeId: number | null;
  renamingId: number | null;
  setRenamingId: (id: number | null) => void;
  open: (listId: number) => Promise<void>;
}

/** A Space's filter lists under the hairline guide, then "New filter list". */
export function FilterListTree({ space, activeId, renamingId, setRenamingId, open }: Props) {
  const lists = space.lists;
  const { containerRef, item, handle, dropLineY } = useDragReorder<HTMLDivElement>((from, to) => {
    void attempt(api.listMove(lists[from].id, to), "Could not move the filter list");
  });

  const create = async () => {
    const id = await attempt(api.listCreate(space.listId, NEW_LIST_NAME, null), "Could not create the filter list");
    if (id === null) return;
    await open(id);
    setRenamingId(id);
  };

  return (
    <div className="sb-tree" ref={containerRef}>
      {lists.map((list, index) => (
        <FilterListRow
          key={list.id}
          list={list}
          active={list.id === activeId}
          renaming={list.id === renamingId}
          onOpen={() => void open(list.id)}
          onRename={() => setRenamingId(list.id)}
          onRenameDone={() => setRenamingId(null)}
          dragProps={{ ...item(index), ...handle(index) }}
        />
      ))}
      <button type="button" className="sb-new-list" onClick={() => void create()}>
        <span className="sb-new-list__plus" aria-hidden="true">
          +
        </span>
        {NEW_LIST_NAME}
      </button>
      <DropLine y={dropLineY} />
    </div>
  );
}

interface RowProps {
  list: ListNode;
  active: boolean;
  renaming: boolean;
  onOpen: () => void;
  onRename: () => void;
  onRenameDone: () => void;
  dragProps: HTMLAttributes<HTMLElement>;
}

function FilterListRow({ list, active, renaming, onOpen, onRename, onRenameDone, dragProps }: RowProps) {
  const openMenu = (e: MouseEvent) => {
    e.preventDefault();
    void api.popupMenu([
      { text: "Rename", run: onRename },
      { text: "Duplicate", run: () => void attempt(api.listDuplicate(list.id), "Could not duplicate the filter list") },
      { text: "Pin to Home", enabled: false },
      "separator",
      { text: "Delete", run: () => void deleteList(list) },
    ]);
  };
  const icon = <Funnel size={15} strokeWidth={2} aria-hidden="true" />;
  if (renaming) {
    return (
      <div className="sb-row sb-list" {...dragProps}>
        {icon}
        <InlineRename
          value={list.name}
          label="Filter list name"
          onSave={(name) => void attempt(api.listRename(list.id, name), "Could not rename the filter list")}
          onDone={onRenameDone}
        />
      </div>
    );
  }
  return (
    <button
      type="button"
      className="sb-row sb-list"
      data-state={active ? "active" : undefined}
      aria-current={active ? "page" : undefined}
      onClick={onOpen}
      onContextMenu={openMenu}
      {...dragProps}
    >
      {icon}
      <span className="sb-row__label">{list.name}</span>
      <ListCount list={list} />
    </button>
  );
}

async function deleteList(list: ListNode) {
  const confirmed = await useStore.getState().confirm({
    title: `Delete the filter list “${list.name}”?`,
    text: "Its conditions, its views and its pins go. The issues stay in Jira and in the Space.",
    action: "Delete Filter List",
  });
  if (confirmed) await attempt(api.listDelete(list.id), "Could not delete the filter list");
}
