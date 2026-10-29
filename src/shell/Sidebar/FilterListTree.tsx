import { type HTMLAttributes, type MouseEvent, useRef, useState } from "react";
import { api, type ListNode, type SpaceNode } from "../../api";
import { InlineRename } from "../../components/InlineRename";
import { Funnel } from "lucide-react";
import { LookPopover } from "../../components/look/LookPopover";
import { Tile } from "../../components/look/Tile";
import { usePinOf } from "../../queries/pins";
import { attempt } from "../../state/requests";
import { useStore } from "../../state/store";
import { DropLine } from "./DropLine";
import { ListCount } from "./ListCount";
import { useDragReorder } from "./useDragReorder";

export const NEW_LIST_NAME = "New filter list";

interface Props {
  space: SpaceNode;
  activeId: number | null;
  renamingId: number | null;
  setRenamingId: (id: number | null) => void;
  open: (listId: number) => Promise<void>;
}

/** Adds a filter list to the Space, opens it and starts renaming it. */
export async function createList(space: SpaceNode, open: (listId: number) => Promise<void>, setRenamingId: (id: number | null) => void) {
  const id = await attempt(api.listCreate(space.listId, NEW_LIST_NAME, null), "Could not create the filter list");
  if (id === null) return;
  await open(id);
  setRenamingId(id);
}

/** A Space's filter lists under the hairline guide; "New filter list" only while it has none (the Space row's
 *  + adds more). */
export function FilterListTree({ space, activeId, renamingId, setRenamingId, open }: Props) {
  const lists = space.lists;
  const { containerRef, item, handle, dropLineY } = useDragReorder<HTMLDivElement>((from, to) => {
    void attempt(api.listMove(lists[from].id, to), "Could not move the filter list");
  });

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
      {lists.length === 0 && (
        <button type="button" className="sb-new-list" onClick={() => void createList(space, open, setRenamingId)}>
          <span className="sb-new-list__plus" aria-hidden="true">
            +
          </span>
          {NEW_LIST_NAME}
        </button>
      )}
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
  const pin = usePinOf("filterList", String(list.id));
  const openMenu = (e: MouseEvent) => {
    e.preventDefault();
    void api.popupMenu([
      { text: "Rename", run: onRename },
      { text: "Duplicate", run: () => void attempt(api.listDuplicate(list.id), "Could not duplicate the filter list") },
      pin
        ? { text: "Remove from Home", run: () => void attempt(api.pinRemove(pin.id), "Could not remove the filter list from Home") }
        : { text: "Add to Home", run: () => void attempt(api.pinCreate("filterList", String(list.id), "untilUnpinned", null), "Could not add the filter list to Home") },
      "separator",
      { text: "Delete", run: () => void deleteList(list) },
    ]);
  };
  const icon = <Tile icon={list.icon} color={list.color} fallback={Funnel} size={list.color ? 16 : 15} />;
  const [picking, setPicking] = useState(false);
  const iconRef = useRef<HTMLButtonElement>(null);
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
    <div className="sb-list-row" {...dragProps}>
      <button
        type="button"
        className="sb-row sb-list"
        data-state={active ? "active" : undefined}
        aria-current={active ? "page" : undefined}
        onClick={onOpen}
        onContextMenu={openMenu}
      >
        <span className="sb-list__icon-slot" aria-hidden="true" />
        <span className="sb-row__label">{list.name}</span>
        <ListCount list={list} />
      </button>
      {/* The icon sits over the row's icon slot: a click on it changes the look (ClickUp style). */}
      <button
        ref={iconRef}
        type="button"
        className="sb-list__icon"
        title="Icon and color"
        aria-label={`Change the icon and color of ${list.name}`}
        aria-expanded={picking}
        onClick={() => setPicking((was) => !was)}
      >
        {icon}
      </button>
      {picking && (
        <LookPopover
          title="Icon and color"
          look={{ icon: list.icon, color: list.color }}
          fallback={Funnel}
          colorOptional
          anchor={iconRef}
          onChange={(look) => void attempt(api.listSetLook(list.id, look.icon, look.color), "Could not change the icon")}
          onClose={() => setPicking(false)}
        />
      )}
    </div>
  );
}

async function deleteList(list: ListNode) {
  const confirmed = await useStore.getState().confirm({
    title: `Delete the filter list “${list.name}”?`,
    text: "Its conditions, its views and its bookmarks go. The issues stay in Jira and in the Space.",
    action: "Delete Filter List",
  });
  if (confirmed) await attempt(api.listDelete(list.id), "Could not delete the filter list");
}
