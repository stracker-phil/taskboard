/** The query's words, lowercased and each once: "Payment gateway" finds "payment" and "gateway" anywhere. */
export function searchTerms(query: string): string[] {
  return [...new Set(query.toLowerCase().split(/\s+/).filter(Boolean))];
}

/** Every visible occurrence of any term under `root`, case-insensitive, in document order. Overlapping ones
 *  ("pay" in "payment") merge into one, and so do neighbors in the same line apart by spaces only ("payment
 *  gateway"). */
export function findMatches(root: Node, terms: string[]): Range[] {
  return joinNeighbors(occurrences(root, terms));
}

function occurrences(root: Node, terms: string[]): Range[] {
  if (terms.length === 0) return [];
  const ranges: Range[] = [];
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    const text = node as Text;
    if (!isShown(text)) continue;
    for (const [start, end] of spans(text.data.toLowerCase(), terms)) {
      const range = document.createRange();
      range.setStart(text, start);
      range.setEnd(text, end);
      ranges.push(range);
    }
  }
  return ranges;
}

function isShown(text: Text): boolean {
  const parent = text.parentElement;
  return parent !== null && (parent.checkVisibility?.() ?? true);
}

/** Where the terms occur in `haystack`, sorted, with overlaps merged. */
function spans(haystack: string, terms: string[]): [number, number][] {
  const found: [number, number][] = [];
  for (const term of terms) {
    for (let at = haystack.indexOf(term); at >= 0; at = haystack.indexOf(term, at + 1)) found.push([at, at + term.length]);
  }
  found.sort((a, b) => a[0] - b[0]);
  const merged: [number, number][] = [];
  for (const span of found) {
    const last = merged[merged.length - 1];
    if (last && span[0] <= last[1]) last[1] = Math.max(last[1], span[1]);
    else merged.push([...span]);
  }
  return merged;
}

/** A run of non-inline layout ends a line: a paragraph, a list item, a flex child. */
function lineOf(node: Node): Element | null {
  let element = node.parentElement;
  while (element && getComputedStyle(element).display === "inline") element = element.parentElement;
  return element;
}

function joinNeighbors(ranges: Range[]): Range[] {
  const joined: Range[] = [];
  for (const range of ranges) {
    const last = joined[joined.length - 1];
    if (last && apartBySpaces(last, range)) last.setEnd(range.endContainer, range.endOffset);
    else joined.push(range);
  }
  return joined;
}

function apartBySpaces(before: Range, after: Range): boolean {
  if (lineOf(before.endContainer) !== lineOf(after.startContainer)) return false;
  const between = document.createRange();
  between.setStart(before.endContainer, before.endOffset);
  between.setEnd(after.startContainer, after.startOffset);
  return /^[ \t\u00a0]*$/.test(between.toString());
}
