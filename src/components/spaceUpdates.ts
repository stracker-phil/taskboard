import { api, type SpaceNode, type SpaceUpdate } from "../api";
import { attempt } from "../state/requests";
import type { Look } from "./look/Tile";

/** A Space's update as it stands: a name equal to the project's stays unset, so it follows the project's. */
export function spaceUpdateOf(space: SpaceNode): SpaceUpdate {
  return { name: space.name === space.projectName ? null : space.name, icon: space.icon, color: space.color, client: space.client };
}

export function renameSpace(space: SpaceNode, name: string) {
  void attempt(api.spaceUpdate(space.projectId, { ...spaceUpdateOf(space), name }), "Could not rename the Space");
}

export function setSpaceLook(space: SpaceNode, look: Look) {
  void attempt(api.spaceUpdate(space.projectId, { ...spaceUpdateOf(space), ...look }), "Could not change the Space's icon");
}
