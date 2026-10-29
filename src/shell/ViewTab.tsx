import type { HTMLAttributes, MouseEvent } from "react";
import { api, type View } from "../api";
import { InlineRename } from "../components/InlineRename";
import { Tile } from "../components/look/Tile";
import { attempt } from "../state/requests";
import { LAYOUTS, LayoutTile } from "./viewLayouts";

interface Props {
  view: View;
  /** The open view: a click on its name renames it, one on its tile edits the look. */
  active: boolean;
  renaming: boolean;
  onOpen: () => void;
  onRename: () => void;
  onRenameDone: () => void;
  onEditLook: (tile: HTMLElement) => void;
  onMenu: (e: MouseEvent) => void;
  /** The drag-to-reorder item and handle props. */
  dragProps: HTMLAttributes<HTMLElement> & { "data-drag-index": number; "data-dragging"?: boolean };
}

/** One view tab: its tile (an own look, else the layout's) and name. */
export function ViewTab({ view, active, renaming, onOpen, onRename, onRenameDone, onEditLook, onMenu, dragProps }: Props) {
  const tile = (
    <span
      className="view-tab__look"
      data-view-look={view.id}
      onClick={(e) => {
        if (!active) return;
        e.stopPropagation();
        onEditLook(e.currentTarget);
      }}
    >
      {view.icon || view.color ? (
        <Tile icon={view.icon} color={view.color} fallback={LAYOUTS[view.layout].icon} size={view.color ? 18 : 15} />
      ) : (
        <LayoutTile layout={view.layout} />
      )}
    </span>
  );
  if (renaming) {
    return (
      <div className="view-tab" data-active={active || undefined} {...dragProps}>
        {tile}
        <InlineRename
          value={view.name}
          label="View name"
          onSave={(name) => void attempt(api.viewRename(view.id, name), "Could not rename the view")}
          onDone={onRenameDone}
        />
      </div>
    );
  }
  return (
    <button
      type="button"
      role="tab"
      className="view-tab"
      aria-selected={active}
      data-active={active || undefined}
      title={active ? "Click to rename, drag to move" : "Drag to move"}
      onClick={active ? onRename : onOpen}
      onDoubleClick={onRename}
      onContextMenu={onMenu}
      data-view-id={view.id}
      {...dragProps}
    >
      {tile}
      {view.name}
    </button>
  );
}
