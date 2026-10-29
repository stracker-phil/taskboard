import type { SpaceNode } from "../api";
import { LookButton } from "./look/LookButton";
import { setSpaceLook } from "./spaceUpdates";
import { SpaceTile } from "./SpaceTile";

interface Props {
  space: SpaceNode;
  size: 20 | 22 | 28;
  className: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Set while the Space isn't selected: a click selects it instead. */
  onSelect?: () => void;
}

/** A Space's tile that edits the Space's icon and color. */
export function SpaceLookButton({ space, size, className, open, onOpenChange, onSelect }: Props) {
  return (
    <LookButton
      label={`Icon and color of ${space.name}`}
      className={className}
      look={{ icon: space.icon, color: space.color ?? "1" }}
      fallback={space.name.charAt(0)}
      open={open}
      onOpenChange={onOpenChange}
      onSelect={onSelect}
      onChange={(look) => setSpaceLook(space, look)}
    >
      <SpaceTile space={space} size={size} />
    </LookButton>
  );
}
