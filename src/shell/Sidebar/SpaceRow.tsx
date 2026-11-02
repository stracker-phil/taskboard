import { DisclosureChevron } from "../../components/DisclosureChevron";
import { Plus } from "lucide-react";
import { useState, type HTMLAttributes, type MouseEvent } from "react";
import { api, type SpaceNode } from "../../api";
import { useShortcuts } from "../../commands/useShortcuts";
import { InlineRename } from "../../components/InlineRename";
import { openContextMenu } from "../../components/menu";
import { SpaceLookButton } from "../../components/SpaceLookButton";
import { renameSpace } from "../../components/spaceUpdates";
import { attempt } from "../../state/requests";
import { useStore } from "../../state/store";
import { copyAppLink } from "../../views/list/useIssueActions";

interface Props {
  space: SpaceNode;
  /** The Space itself is the current level: a click on its name renames it, one on its tile edits the look. */
  active: boolean;
  /** One of its filter lists is the current level. */
  holdsActive: boolean;
  expanded: boolean;
  renaming: boolean;
  onOpen: () => void;
  /** `others`: ⌥-click, collapse every other Space. */
  onToggle: (others: boolean) => void;
  onRename: () => void;
  onRenameDone: () => void;
  onEditClient: () => void;
  onAddList: () => void;
  /** Pointer handlers that drag the row. */
  dragHandle: HTMLAttributes<HTMLElement>;
}

/** A Space in the sidebar: chevron, tile, name and, once it has filter lists, a + for another; the row opens it,
 *  the chevron folds its lists. */
export function SpaceRow({ space, active, holdsActive, expanded, renaming, onOpen, onToggle, onRename, onRenameDone, onEditClient, onAddList, dragHandle }: Props) {
  const [editingLook, setEditingLook] = useState(false);
  const openMenu = (e: MouseEvent) =>
    openContextMenu(
      e,
      [
        { text: "Rename", run: onRename },
        { text: "Icon and Color…", run: () => setEditingLook(true) },
        { text: "Client label…", run: onEditClient },
        { text: "Space Settings…", run: () => useStore.getState().setSpaceSheet(space.projectId) },
        { text: "Copy Taskboard Link", run: () => copyAppLink({ kind: "level", listId: space.listId, viewId: null }, `Copied the Taskboard link to ${space.name}`) },
        "separator",
        { text: "Stop tracking", danger: true, run: () => void stopTracking(space) },
      ],
      space.name,
    );
  const state = active ? "active" : holdsActive ? "holds-active" : undefined;
  const { title } = useShortcuts();
  return (
    <div className="sb-row sb-space" data-state={state} onContextMenu={openMenu} {...dragHandle}>
      <button
        type="button"
        className="sb-space__chevron"
        aria-label={`${expanded ? "Collapse" : "Expand"} ${space.name}`}
        aria-expanded={expanded}
        onClick={(e) => onToggle(e.altKey)}
      >
        <DisclosureChevron open={expanded} />
      </button>
      <SpaceLookButton
        space={space}
        size={22}
        className="sb-space__tile"
        open={editingLook}
        onOpenChange={setEditingLook}
        onSelect={active ? undefined : onOpen}
      />
      {renaming ? (
        <div className="sb-space__main">
          <InlineRename value={space.name} label="Space name" onSave={(name) => renameSpace(space, name)} onDone={onRenameDone} />
        </div>
      ) : (
        <button
          type="button"
          className="sb-space__main"
          aria-current={active ? "page" : undefined}
          title={active ? "Click to rename" : undefined}
          onClick={active ? onRename : onOpen}
          onDoubleClick={onRename}
        >
          <span className="sb-row__label">{space.name}</span>
        </button>
      )}
      {!renaming && space.lists.length > 0 && (
        <button type="button" className="sb-space__add" aria-label={`New filter list in ${space.name}`} title={active || holdsActive ? title("New filter list", "list.new") : "New filter list"} onClick={onAddList}>
          <Plus size={14} strokeWidth={2} aria-hidden="true" />
        </button>
      )}
    </div>
  );
}

/** Hides the Space and stops syncing its project; tracking it again brings back its lists and views. */
async function stopTracking(space: SpaceNode) {
  const confirmed = await useStore.getState().confirm({
    title: `Stop tracking “${space.name}”?`,
    text: "Its issues leave the sidebar and are removed from this Mac; the sync skips the project. Its filter lists, views and what you've read are kept for when you track it again.",
    action: "Stop Tracking",
  });
  if (confirmed) await attempt(api.projectTrack(space.projectId, false), "Could not stop tracking the Space");
}
