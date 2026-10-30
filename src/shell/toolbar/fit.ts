/** Toolbar items that can move into the "…" menu, in the order they go (the owner's choice): the Me chip last. */
export const OVERFLOW_ORDER = ["columns", "subtasks", "filter", "group", "me"] as const;
export type Collapsible = (typeof OVERFLOW_ORDER)[number];

const GAP = 8;
/** The text filter shrinks to this and never moves. */
export const TEXT_FILTER_MIN = 96;
const MORE_BUTTON = 30;
/** About the width of Revert View and Save View, which never move into the "…" menu. */
export const DRAFT_ACTIONS = 30 + GAP + 84;

/** Which items to hide so the rest fits in `available` px. `widths` are the items' natural widths; an item
 *  never measured counts as 0, so it is only hidden once its width is known. `absent` items aren't in this bar
 *  (the Board has no Subtasks button) and never count. The room for Revert View and Save View is always kept, so
 *  changing a filter never folds an item away (docs/ux.md → Stable controls). */
export function overflowing(available: number, widths: Partial<Record<Collapsible, number>>, absent: readonly Collapsible[] = []): Collapsible[] {
  const hidden: Collapsible[] = [];
  const order = OVERFLOW_ORDER.filter((item) => !absent.includes(item));
  const needs = () => {
    const shown = order.filter((item) => !hidden.includes(item));
    const items = shown.reduce((sum, item) => sum + (widths[item] ?? 0) + GAP, 0);
    return items + TEXT_FILTER_MIN + DRAFT_ACTIONS + GAP + (hidden.length > 0 ? MORE_BUTTON + GAP : 0);
  };
  for (const item of order) {
    if (needs() <= available) break;
    hidden.push(item);
  }
  return hidden;
}
