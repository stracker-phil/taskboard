import { api, type Nav, type SpaceNode } from "../../api";
import { Disclosure } from "../../components/Disclosure";
import { attempt } from "../../state/requests";
import { DropLine } from "./DropLine";
import { createList, FilterListTree } from "./FilterListTree";
import { SpaceRow, type SpaceEditFocus } from "./SpaceRow";
import { useDragReorder } from "./useDragReorder";

interface Props {
  spaces: SpaceNode[];
  nav: Nav;
  /** The Space holding the current level, if any. */
  activeSpace: SpaceNode | null;
  open: (listId: number) => Promise<void>;
  setExpanded: (expanded: number[]) => void;
  renamingId: number | null;
  setRenamingId: (id: number | null) => void;
  onEdit: (space: SpaceNode, focus: SpaceEditFocus) => void;
}

/** "Spaces", then each Space with its filter lists while expanded; Spaces are added in Settings → Projects. */
export function SpacesSection({ spaces, nav, activeSpace, open, setExpanded, renamingId, setRenamingId, onEdit }: Props) {
  const { containerRef, item, handle, dropLineY } = useDragReorder<HTMLElement>((from, to) => {
    void attempt(api.spaceMove(spaces[from].projectId, to), "Could not move the Space");
  });
  const currentId = nav.location?.listId ?? null;

  const toggle = (space: SpaceNode, expanded: boolean, others: boolean) => {
    if (others) setExpanded([space.listId]);
    else if (expanded) setExpanded(nav.expanded.filter((id) => id !== space.listId));
    else setExpanded([...nav.expanded, space.listId]);
  };

  return (
    <nav className="sb-spaces" aria-label="Spaces" ref={containerRef}>
      <div className="sb-spaces__header">
        <span className="sb-spaces__title">Spaces</span>
      </div>
      {spaces.map((space, index) => {
        const isActiveSpace = space.listId === activeSpace?.listId;
        const expanded = isActiveSpace || nav.expanded.includes(space.listId);
        return (
          <div key={space.projectId} className="sb-space-group" {...item(index)}>
            <SpaceRow
              space={space}
              active={space.listId === currentId}
              holdsActive={isActiveSpace && space.listId !== currentId}
              expanded={expanded}
              renaming={renamingId === space.listId}
              onOpen={() => void open(space.listId)}
              onToggle={(others) => toggle(space, expanded, others)}
              onRename={() => setRenamingId(space.listId)}
              onRenameDone={() => setRenamingId(null)}
              onEdit={(focus) => onEdit(space, focus)}
              onAddList={() => void createList(space, open, setRenamingId)}
              dragHandle={handle(index)}
            />
            <Disclosure open={expanded}>
              <FilterListTree space={space} activeId={currentId} renamingId={renamingId} setRenamingId={setRenamingId} open={open} />
            </Disclosure>
          </div>
        );
      })}
      <DropLine y={dropLineY} />
    </nav>
  );
}
