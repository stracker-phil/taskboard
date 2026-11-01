/** How many items, in order, fit `available` px with `gap` between them; an item never measured counts as 0. */
export function fitCount(available: number, widths: number[], gap: number): number {
  let used = 0;
  for (const [index, width] of widths.entries()) {
    used += width + (index > 0 ? gap : 0);
    if (used > available) return index;
  }
  return widths.length;
}
