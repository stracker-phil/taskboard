import type { CSSProperties } from "react";
import { LIT, SHAPE } from "./hierarchy";

const SHAPES: Record<string, string> = { [SHAPE.pass]: "pass", [SHAPE.branch]: "branch", [SHAPE.last]: "last", [SHAPE.none]: "none" };
const LITS: Record<string, string> = { [LIT.through]: " tree-guide--through", [LIT.end]: " tree-guide--end", [LIT.off]: "" };

/** A nested row's indent guides, one per level above it (`treeGuides`). The branch into the hovered or selected row
 *  is its own element, always there so it can grow in and shrink away. */
export function TreeGuides({ guides }: { guides: string }) {
  const levels = guides.match(/../g) ?? [];
  return (
    <span className="tree-guides" aria-hidden="true">
      {levels.map(([shape, lit], level) => (
        <span key={level} className={`tree-guide tree-guide--${SHAPES[shape]}${LITS[lit]}`} style={{ "--level": level } as CSSProperties}>
          <span className="tree-guide__branch" />
        </span>
      ))}
    </span>
  );
}
