import type { CSSProperties } from "react";
import { GUIDE } from "./hierarchy";

const CLASS: Record<string, string> = { [GUIDE.line]: "tree-guide", [GUIDE.path]: "tree-guide tree-guide--path", [GUIDE.end]: "tree-guide tree-guide--end" };

/** A nested row's indent guides, one line per level above it (`treeGuides`). */
export function TreeGuides({ marks }: { marks: string }) {
  return (
    <span className="tree-guides" aria-hidden="true">
      {[...marks].map((mark, level) => (
        <span key={level} className={CLASS[mark]} style={{ "--level": level } as CSSProperties} />
      ))}
    </span>
  );
}
