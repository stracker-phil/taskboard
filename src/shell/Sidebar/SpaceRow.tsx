import type { HTMLAttributes, MouseEvent } from "react";
import { api, type SpaceNode } from "../../api";
import { InlineRename } from "../../components/InlineRename";
import { SpaceTile } from "../../components/SpaceTile";
import { attempt } from "../../state/requests";
import { useStore } from "../../state/store";

/** Which part of a Space the small Space popover opens on. */
export type SpaceEditFocus = "appearance" | "client";

interface Props {
  space: SpaceNode;
  /** The Space itself is the current level. */
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
  onEdit: (focus: SpaceEditFocus) => void;
  /** Pointer handlers that drag the row. */
  dragHandle: HTMLAttributes<HTMLElement>;
}

function rename(space: SpaceNode, name: string) {
  const { icon, color, client } = space;
  void attempt(api.spaceUpdate(space.projectId, { name, icon, color, client }), "Could not rename the Space");
}

/** A Space in the sidebar: chevron, tile, name and open count; the row opens it, the chevron folds its lists. */
export function SpaceRow({ space, active, holdsActive, expanded, renaming, onOpen, onToggle, onRename, onRenameDone, onEdit, dragHandle }: Props) {
  const openMenu = (e: MouseEvent) => {
    e.preventDefault();
    void api.popupMenu([
      { text: "Rename", run: onRename },
      { text: "Colour and icon…", run: () => onEdit("appearance") },
      { text: "Client label…", run: () => onEdit("client") },
      "separator",
      { text: "Stop tracking", run: () => void stopTracking(space) },
    ]);
  };
  const state = active ? "active" : holdsActive ? "holds-active" : undefined;
  return (
    <div className="sb-row sb-space" data-state={state} onContextMenu={openMenu} {...dragHandle}>
      <button
        type="button"
        className="sb-space__chevron"
        aria-label={`${expanded ? "Collapse" : "Expand"} ${space.name}`}
        aria-expanded={expanded}
        onClick={(e) => onToggle(e.altKey)}
      >
        <span aria-hidden="true">{expanded ? "▾" : "▸"}</span>
      </button>
      {renaming ? (
        <div className="sb-space__main">
          <SpaceTile space={space} size={22} />
          <InlineRename value={space.name} label="Space name" onSave={(name) => rename(space, name)} onDone={onRenameDone} />
        </div>
      ) : (
        <button type="button" className="sb-space__main" aria-current={active ? "page" : undefined} onClick={onOpen}>
          <SpaceTile space={space} size={22} />
          <span className="sb-row__label">{space.name}</span>
          <span className="sb-row__count">{space.openCount}</span>
        </button>
      )}
    </div>
  );
}

/** Hides the Space and stops syncing its project; tracking it again brings back its lists and views. */
async function stopTracking(space: SpaceNode) {
  const confirmed = await useStore.getState().confirm({
    title: `Stop tracking “${space.name}”?`,
    text: "Its issues leave the sidebar and the next sync skips the project. Its filter lists and views are kept for when you track it again.",
    action: "Stop Tracking",
  });
  if (confirmed) await attempt(api.projectTrack(space.projectId, false), "Could not stop tracking the Space");
}
