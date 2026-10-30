import { DEFAULT_LIST_ICON } from "../components/look/icons";
import { useState } from "react";
import { api } from "../api";
import { InlineRename } from "../components/InlineRename";
import { LookButton } from "../components/look/LookButton";
import { Tile } from "../components/look/Tile";
import { SpaceLookButton } from "../components/SpaceLookButton";
import { renameSpace } from "../components/spaceUpdates";
import { SpaceTile } from "../components/SpaceTile";
import { useNavGo } from "../queries/sidebar";
import { attempt } from "../state/requests";
import { HOME_ICONS, type Level } from "./levels";

/** The level's tile and name. A Space's and a filter list's name is renamed with a click, their tile edits the
 *  look. */
export function HeaderTitle({ level }: { level: Level }) {
  const open = useNavGo();
  const [editingLook, setEditingLook] = useState(false);
  switch (level.kind) {
    case "space": {
      const { space } = level;
      return (
        <>
          <SpaceLookButton space={space} size={28} className="content-header__tile" open={editingLook} onOpenChange={setEditingLook} />
          <EditableTitle key={space.listId} name={space.name} label="Space name" onSave={(name) => renameSpace(space, name)} />
        </>
      );
    }
    case "list": {
      const { space, node } = level;
      return (
        <>
          <button type="button" className="content-header__crumb" onClick={() => void open(space.listId)}>
            <SpaceTile space={space} size={20} />
            {space.name}
          </button>
          <span className="content-header__slash" aria-hidden="true">
            /
          </span>
          <LookButton
            label={`Icon and color of ${node.name}`}
            className="content-header__tile"
            look={{ icon: node.icon, color: node.color }}
            fallback={DEFAULT_LIST_ICON}
            colorOptional
            open={editingLook}
            onOpenChange={setEditingLook}
            onChange={(look) => void attempt(api.listSetLook(node.id, look.icon, look.color), "Could not change the icon")}
          >
            <Tile icon={node.icon} color={node.color} fallback={DEFAULT_LIST_ICON} size={node.color ? 20 : 15} />
          </LookButton>
          <EditableTitle
            key={node.id}
            name={node.name}
            label="Filter list name"
            onSave={(name) => void attempt(api.listRename(node.id, name), "Could not rename the filter list")}
          />
        </>
      );
    }
    case "home": {
      const Icon = HOME_ICONS[level.node.kind];
      return (
        <>
          <Icon className="content-header__icon" size={20} strokeWidth={1.8} aria-hidden="true" />
          <h1 className="content-header__title">{level.node.name}</h1>
        </>
      );
    }
  }
}

/** The title; a click turns it into a field. */
function EditableTitle({ name, label, onSave }: { name: string; label: string; onSave: (name: string) => void }) {
  const [renaming, setRenaming] = useState(false);
  if (renaming) return <InlineRename className="content-header__rename" value={name} label={label} onSave={onSave} onDone={() => setRenaming(false)} />;
  return (
    <h1 className="content-header__title">
      <button type="button" className="content-header__name" title="Click to rename" onClick={() => setRenaming(true)}>
        {name}
      </button>
    </h1>
  );
}
